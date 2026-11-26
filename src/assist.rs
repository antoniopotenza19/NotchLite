//! Opt-in coordinator. Call audio requires a new explicit confirmation per session.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::assist_metrics::{MetricsWorker, ResourceSnapshot};
use crate::assist_overlay::*;
use crate::call_audio::{AudioMonitor, AudioSnapshot};
use crate::codex_bridge::{self, Bridge, Connection, Snapshot, Status};
use crate::{store, worker};

pub const DASHBOARD_KEY: &str = "codex_dashboard";
pub const CALL_KEY: &str = "call_panel";
pub const MONITOR_KEY: &str = "assistant_monitor";
pub const SOCKET_KEY: &str = "codex_socket";
pub const CLI_KEY: &str = "codex_cli";

pub fn enabled(map: &BTreeMap<String, String>, key: &str) -> bool {
    map.get(key).is_some_and(|value| value == "1")
}

#[derive(Default)]
struct Attention {
    message: String,
    status: Option<Status>,
    completed: bool,
    unread: bool,
    until: Option<Instant>,
}

fn update_attention(previous: &mut Attention, thread: &codex_bridge::ThreadView, selected: bool, now: Instant) {
    let initialized = previous.status.is_some();
    let message = initialized && previous.message != thread.last_message && !thread.last_message.is_empty();
    let completed = initialized && !previous.completed && thread.completed;
    let needs_input = matches!(thread.status, Status::WaitingInput | Status::WaitingApproval);
    if message || completed {
        previous.unread = !selected;
        previous.until = Some(now + Duration::from_secs(5));
    }
    if selected { previous.unread = false; }
    if needs_input { previous.until = Some(now + Duration::from_secs(5)); }
    previous.message = thread.last_message.clone();
    previous.status = Some(thread.status.clone());
    previous.completed = thread.completed;
}

pub struct Assistant {
    overlays: OverlayManager,
    config: OverlayConfig,
    availability: OverlayAvailability,
    socket: Option<PathBuf>,
    cli: Option<PathBuf>,
    server_start: Option<crate::codex_server::StartWorker>,
    workspace: String,
    bridge: Option<Bridge>,
    bridge_attempted: bool,
    codex: Snapshot,
    metrics: Option<MetricsWorker>,
    resources: ResourceSnapshot,
    monitor: Option<AudioMonitor>,
    monitor_attempted: bool,
    audio: Option<AudioSnapshot>,
    call_session: Option<crate::call_session::CallSession>,
    call_transcript: String,
    call_suggestion: String,
    call_notice: Option<String>,
    selected_thread: Option<String>,
    observed_created_thread: Option<String>,
    selected_target: Option<u64>,
    attention: BTreeMap<String, Attention>,
    manual_call: bool,
    dismissed_call: Option<u64>,
    note: Option<String>,
    capture_exclusion: bool,
}

impl Assistant {
    /// Revoke capture before shell teardown destroys its visible indicator.
    pub fn stop_before_exit(&mut self) { self.stop_call(); }

    pub fn new() -> Self {
        Self {
            overlays: OverlayManager::new(), config: OverlayConfig::default(), availability: OverlayAvailability::Off,
            socket: None, bridge: None, bridge_attempted: false,
            cli: None, server_start: None, workspace: String::new(),
            codex: Snapshot::unavailable("Pannello chat disattivato"), metrics: None, resources: ResourceSnapshot::default(),
            monitor: None, monitor_attempted: false, audio: None,
            call_session: None, call_transcript: String::new(), call_suggestion: String::new(), call_notice: None,
            selected_thread: None, selected_target: None, attention: BTreeMap::new(), manual_call: false,
            observed_created_thread: None,
            dismissed_call: None, note: None,
            capture_exclusion: true,
        }
    }

    pub fn sync(&mut self, settings: &BTreeMap<String, String>, hwnd: HWND) -> Result<(), String> {
        // A settings reload can reapply affinity or recreate/hide a panel. Revoke
        // audio before that work, including when the OS rejects the new policy.
        let next_capture_exclusion = crate::window_policy::current().exclude_capture;
        let privacy_changed = self.capture_exclusion != next_capture_exclusion;
        let had_call = self.call_session.is_some();
        self.stop_call();
        self.capture_exclusion = next_capture_exclusion;
        if privacy_changed || had_call {
            self.call_notice = Some("Impostazioni aggiornate: conferma nuovamente la trascrizione per riprendere l'ascolto.".into());
        }
        let config = OverlayConfig {
            codex_dashboard: enabled(settings, DASHBOARD_KEY), call_panel: enabled(settings, CALL_KEY),
            monitor_device: settings.get(MONITOR_KEY).filter(|v| !v.is_empty() && *v != "auto").cloned(),
        };
        let socket = settings.get(SOCKET_KEY).filter(|value| !value.is_empty()).map(PathBuf::from);
        let cli = settings.get(CLI_KEY).filter(|value| !value.is_empty()).map(PathBuf::from);
        if config.call_panel != self.config.call_panel || config.monitor_device != self.config.monitor_device
            || socket != self.socket || cli != self.cli { self.stop_call(); }
        if config.codex_dashboard != self.config.codex_dashboard || socket != self.socket || cli != self.cli {
            self.bridge = None; self.bridge_attempted = false; self.attention.clear();
            self.selected_thread = None;
            self.observed_created_thread = None;
        }
        if config.call_panel != self.config.call_panel {
            self.monitor = None; self.monitor_attempted = false; self.audio = None;
            self.manual_call = config.call_panel; self.dismissed_call = None;
        }
        self.socket = socket;
        self.cli = cli;
        // Resolve the default folder on settings sync, never in painting.
        if self.workspace.is_empty() {
            self.workspace = std::env::current_dir().ok().and_then(|p| p.into_os_string().into_string().ok()).unwrap_or_default();
        }
        self.config = config.clone();
        self.overlays.set_wake_window(hwnd);
        let previous = self.availability;
        self.availability = match self.overlays.sync(config) {
            Ok(availability) => availability,
            Err(error) => {
                self.availability = OverlayAvailability::Off;
                self.manage_workers();
                return Err(format!("Pannelli assistente: {error}"));
            }
        };
        if self.availability != previous && (self.config.codex_dashboard || self.config.call_panel) {
            let message = match self.availability {
                OverlayAvailability::WaitingForSecondary => Some("Collega un secondo monitor: i pannelli non vengono spostati sul primario."),
                OverlayAvailability::ChooseSecondary => Some("Scegli il monitor secondario in Settings > Assistant monitor."),
                OverlayAvailability::InsufficientSpace => Some("Spazio insufficiente sul monitor scelto per entrambi i pannelli."),
                _ => None,
            };
            if let Some(message) = message {
                worker::peek(worker::Peek { icon: "\u{E7F4}".into(), emoji: false, accent: crate::theme::warn(),
                    title: "Assistant panels".into(), body: message.into(), seconds: 5.0 });
            }
        }
        self.manage_workers();
        self.render();
        Ok(())
    }

    fn manage_workers(&mut self) {
        if self.availability != OverlayAvailability::Ready {
            self.stop_call();
            self.bridge = None; self.bridge_attempted = false;
            self.metrics = None; self.monitor = None; self.monitor_attempted = false;
            self.audio = None;
            return;
        }
        if self.config.codex_dashboard {
            if !self.bridge_attempted {
                self.bridge_attempted = true;
                let result = match &self.cli {
                    Some(cli) => Bridge::start_with_cli(self.socket.clone(), cli.clone()),
                    None => Bridge::start(self.socket.clone()),
                };
                match result {
                    Ok(bridge) => { self.codex = bridge.snapshot(); self.bridge = Some(bridge); self.note = None; }
                    Err(error) => { self.codex = Snapshot::unavailable(error); }
                }
            }
            if self.metrics.is_none() {
                match MetricsWorker::start() {
                    Ok(metrics) => self.metrics = Some(metrics),
                    Err(error) => self.resources.error = Some(error),
                }
            }
        } else { self.bridge = None; self.metrics = None; }
        if self.config.call_panel {
            if !self.monitor_attempted {
                self.monitor_attempted = true;
                match AudioMonitor::start() {
                    Ok(monitor) => self.monitor = Some(monitor),
                    Err(error) => self.note = Some(error),
                }
            }
        } else { self.stop_call(); self.monitor = None; self.audio = None; }
    }

    pub fn display_changed(&mut self) {
        // Never keep recording while a monitor/DPI transition hides the indicator.
        self.stop_call();
        match self.overlays.display_changed() {
            Ok(availability) => self.availability = availability,
            Err(error) => { self.availability = OverlayAvailability::WaitingForSecondary; self.note = Some(error.to_string()); }
        }
        self.manage_workers();
        self.render();
    }

    pub fn tick(&mut self) {
        if let Some(result) = self.server_start.as_ref().and_then(|worker| worker.poll()) {
            self.server_start = None;
            match result {
                Ok(_) => {
                    self.bridge = None; self.bridge_attempted = false;
                    self.manage_workers();
                    self.note = Some("Server locale disponibile. Le chat Desktop già aperte restano nell'app originale; qui puoi creare nuove chat in sola lettura.".into());
                }
                Err(error) => { self.note = Some(error); }
            }
        }
        if crate::assist_overlay::take_topology_dirty() { self.display_changed(); }
        if self.availability != OverlayAvailability::Ready { return; }
        if let Some(bridge) = &self.bridge { self.codex = bridge.snapshot(); }
        if let Some(metrics) = &self.metrics {
            if let Some(snapshot) = metrics.poll() { self.resources = snapshot; }
        }
        if let Some(monitor) = &self.monitor { self.audio = Some(monitor.snapshot()); }
        if self.audio.as_ref().is_some_and(|audio| audio.suggested.is_none()) { self.dismissed_call = None; }
        self.tick_call();
        if !self.selected_target.is_some_and(|key| self.audio.as_ref().is_some_and(|s| s.candidates.iter().any(|c| c.key == key))) {
            self.selected_target = None;
        }
        let ids: BTreeSet<_> = self.codex.threads.iter().map(|thread| thread.id.clone()).collect();
        if self.codex.created_thread_id.is_some() && self.codex.created_thread_id != self.observed_created_thread {
            self.observed_created_thread = self.codex.created_thread_id.clone();
            self.selected_thread = self.codex.created_thread_id.clone().filter(|id| ids.contains(id));
        }
        self.attention.retain(|id, _| ids.contains(id));
        if !self.selected_thread.as_ref().is_some_and(|id| ids.contains(id)) {
            self.selected_thread = self.codex.threads.first().map(|thread| thread.id.clone());
        }
        let now = Instant::now();
        for thread in &self.codex.threads {
            update_attention(self.attention.entry(thread.id.clone()).or_default(), thread,
                self.selected_thread.as_deref() == Some(thread.id.as_str()), now);
        }
        self.render();
    }

    pub fn actions(&mut self) {
        if crate::assist_overlay::take_topology_dirty() { self.display_changed(); }
        for action in self.overlays.drain_actions() {
            let result = self.action(action);
            if let Err(error) = result { self.note = Some(error); }
        }
        self.render();
    }

    fn action(&mut self, action: OverlayAction) -> Result<(), String> {
        match action {
            OverlayAction::StartSharedServer => {
                if self.server_start.is_some() { return Err("Avvio del server già in corso".into()); }
                if self.socket.is_some() { return Err("È configurato un socket personalizzato: avvia quel server dal suo client.".into()); }
                if self.codex.connection == Connection::Connected { return Err("Server già collegato".into()); }
                self.server_start = Some(crate::codex_server::StartWorker::start(self.cli.clone())?);
                self.note = Some("Avvio del server locale condiviso… rimarrà attivo dopo la chiusura del pannello per conservare le chat.".into());
            }
            OverlayAction::CreateThread { workspace } => {
                self.bridge.as_ref().ok_or("Codex non collegato")?.create_thread(&workspace)?;
                self.workspace = workspace;
                self.note = Some("Creazione richiesta: chat in sola lettura, autorizzazioni affidate all'utente. Nessun messaggio viene inviato automaticamente.".into());
            }
            OverlayAction::SelectThread(id) => {
                if !self.codex.threads.iter().any(|t| t.id == id) { return Err("Chat non più disponibile".into()); }
                self.selected_thread = Some(id.clone());
                if let Some(attention) = self.attention.get_mut(&id) { attention.unread = false; }
                self.note = None;
            }
            OverlayAction::SendFollowup { thread_id, text } => {
                self.bridge.as_ref().ok_or("Codex non collegato")?.follow_up(&thread_id, &text)?;
                self.note = Some("Invio richiesto: attendi la conferma del server Codex.".into());
            }
            OverlayAction::AnswerQuestion { key, answers } => {
                self.bridge.as_ref().ok_or("Codex non collegato")?.answer(key, answers)?;
                self.note = Some("Risposta in coda di invio alla richiesta selezionata.".into());
            }
            OverlayAction::Refresh => {
                self.note = None;
                if let Some(bridge) = &self.bridge { bridge.refresh()?; }
                else { self.bridge_attempted = false; self.manage_workers(); }
            }
            OverlayAction::OpenCodex { thread_id } => {
                let uri = thread_uri(&thread_id).ok_or("Identificatore chat Codex non valido")?;
                let uri: Vec<u16> = uri.encode_utf16().chain(Some(0)).collect();
                let result = unsafe { ShellExecuteW(HWND::default(), None, PCWSTR(uri.as_ptr()), None, None, SW_SHOWNORMAL) };
                if result.0 as usize <= 32 { return Err("Impossibile aprire il collegamento nell'app Codex".into()); }
            }
            OverlayAction::ShowCallPanel => { self.manual_call = true; self.dismissed_call = None; }
            OverlayAction::SelectCallTarget { target_key } => {
                if !self.audio.as_ref().is_some_and(|s| s.candidates.iter().any(|c| c.key == target_key)) {
                    return Err("La sessione audio selezionata non è più disponibile".into());
                }
                self.selected_target = Some(target_key); self.manual_call = true;
            }
            OverlayAction::ConfirmCall { candidate_key, context, with_ai } => {
                if let Err(error) = self.start_call(candidate_key, context, with_ai) {
                    self.call_notice = Some(error.clone()); return Err(error);
                }
            }
            OverlayAction::DeclineCall { candidate_key } => {
                self.stop_call();
                if let Some(monitor) = &self.monitor { monitor.decline(candidate_key); }
                self.dismissed_call = Some(candidate_key); self.manual_call = false;
            }
            OverlayAction::StopCall => {
                self.stop_call();
                self.manual_call = true;
                self.call_notice = Some("Ascolto interrotto. Il testo già trascritto resta visibile.".into());
            }
            OverlayAction::CloseCall => {
                self.stop_call();
                self.dismissed_call = self.audio.as_ref().and_then(|s| s.suggested).or(self.selected_target);
                self.manual_call = false;
            }
            OverlayAction::CloseDashboard => {
                // Stop first; even a settings write failure must not reconnect the panel.
                self.bridge = None; self.metrics = None; self.config.codex_dashboard = false;
                self.availability = self.overlays.sync(self.config.clone()).map_err(|e| e.to_string())?;
                let mut settings = store::try_read_kv("settings.txt").map_err(|e| e.to_string())?;
                settings.insert(DASHBOARD_KEY.into(), "0".into());
                store::write_kv("settings.txt", &settings).map_err(|e| e.to_string())?;
                worker::send(worker::ShellCmd::ReloadSettings);
            }
        }
        Ok(())
    }

    fn dashboard(&self) -> DashboardView {
        let now = Instant::now();
        let mut view = DashboardView { selected_id: self.selected_thread.clone(), resources: self.resources.display(), ..Default::default() };
        view.workspace = self.workspace.clone();
        view.can_start_server = self.server_start.is_none() && self.socket.is_none()
            && matches!(self.codex.connection, Connection::Unavailable(_));
        view.can_create_thread = self.codex.connection == Connection::Connected
            && self.codex.account.is_ready() && !self.codex.creating_thread;
        view.connection = match &self.codex.connection {
            Connection::Connecting => "Collegamento a Codex…".into(),
            Connection::Connected => "Codex collegato · server condiviso locale".into(),
            Connection::Unavailable(reason) => reason.clone(),
        };
        if self.codex.connection == Connection::Connected {
            match &self.codex.account {
                codex_bridge::AccountState::Checking => view.connection.push_str(" · verifica accesso ChatGPT…"),
                codex_bridge::AccountState::ChatGpt { .. } => {},
                codex_bridge::AccountState::Unavailable(reason) => { view.connection.push_str("\r\n"); view.connection.push_str(reason); }
            }
        }
        for thread in &self.codex.threads {
            let statuslabel = match thread.status {
                Status::Idle if thread.completed => "Conclusa",
                Status::Idle => "Inattiva",
                Status::Active => "In corso",
                Status::WaitingInput => "Domanda",
                Status::WaitingApproval => "Autorizzazione in Codex",
                Status::Unavailable => "Non disponibile",
            };
            let attention = self.attention.get(&thread.id).is_some_and(|a| a.unread || a.until.is_some_and(|until| until > now));
            view.rows.push(ChatRow { id: thread.id.clone(), title: thread.title.clone(), statuslabel: statuslabel.into(), attention });
        }
        let quotas = self.codex.quota.as_ref().map(|q| q.windows.iter()
            .map(|w| format!("{}: {}% usato", w.label, w.used_percent)).collect::<Vec<_>>().join(" · "));
        view.usage = quotas.filter(|q| !q.is_empty()).unwrap_or_else(|| "Limiti Codex non disponibili".into());
        if let Some(thread) = self.codex.threads.iter().find(|t| Some(&t.id) == self.selected_thread.as_ref()) {
            view.can_answer = self.codex.account.is_ready() && thread.subscription_eligible && thread.subscription_ready;
            view.detail = thread.last_message.clone();
            if !thread.subscription_ready {
                view.detail.push_str("\r\n\r\nCollegamento agli eventi della chat in attesa di conferma.");
                if thread.subscription_error.as_ref().is_some_and(|e| e.reason == "no_rollout") {
                    view.detail.push_str(" La sessione è ancora vuota: invia il primo messaggio dal client che l'ha creata.");
                }
            }
            if let Some(tokens) = &thread.token_usage {
                view.usage.push_str(&format!("\r\nChat: {} token · input {} · output {}", tokens.total, tokens.input, tokens.output));
            }
            view.can_followup = thread.status == Status::Idle && self.codex.connection == Connection::Connected
                && self.codex.account.is_ready() && thread.subscription_eligible && thread.subscription_ready;
            if let Some(request) = thread.questions.first() {
                if request.questions.iter().any(|q| q.is_secret) {
                    view.approval_notice = Some("Domanda con campo riservato: rispondi nell'app Codex.".into());
                } else {
                    // Keep option explanations readable in the scrollable detail,
                    // while combo values remain the exact protocol labels.
                    for question in &request.questions {
                        view.detail.push_str(&format!("\r\n\r\n{}: {}", question.header, question.text));
                        for option in &question.options {
                            if !option.description.is_empty() {
                                view.detail.push_str(&format!("\r\n{}: {}", option.label, option.description));
                            }
                        }
                    }
                    view.question = Some(QuestionForm { key: request.key.clone(), questions: request.questions.iter().map(|q| Prompt {
                        id: q.id.clone(), text: format!("{}: {}", q.header, q.text), options: q.options.iter().map(|o| o.label.clone()).collect(),
                    }).collect() });
                }
            }
            if !thread.approvals.is_empty() {
                view.approval_notice = Some("Richiesta di autorizzazione: controlla e decidi in un client Codex collegato allo stesso server.".into());
            }
        }
        if let Some(note) = self.codex.last_error.as_ref().or(self.note.as_ref()) {
            view.connection.push_str("\r\n"); view.connection.push_str(note);
        }
        view
    }

    fn call_view(&self) -> CallView {
        let suggested = self.audio.as_ref().and_then(|s| s.suggested);
        let key = self.selected_target.or(suggested);
        let candidate = key.and_then(|key| self.audio.as_ref()?.candidates.iter().find(|c| c.key == key));
        let capture = self.audio.as_ref().and_then(|s| s.capture.as_ref());
        let mut view = CallView { visible: self.config.call_panel && (capture.is_some() || self.manual_call || (suggested.is_some() && suggested != self.dismissed_call)),
            provider: "Scegli Trascrivi oppure Trascrivi + AI · audio elaborato localmente".into(),
            can_listen: self.monitor.is_some() && capture.is_none() && self.call_session.is_none(),
            listening: capture.is_some(), candidate_key: candidate.map(|c| c.key), ..Default::default() };
        view.status = if suggested.is_some() { "Possibile chiamata rilevata" } else { "In attesa di una chiamata · solo metadati" }.into();
        view.scope = candidate.map(|c| format!("{} (PID {}) · audio dell'app e processi figli, non una singola scheda.", c.label, c.pid))
            .unwrap_or_else(|| "Nessun ascolto: la rilevazione usa soltanto metadati delle sessioni audio.".into());
        if let Some(audio) = &self.audio {
            view.candidates = audio.candidates.iter().map(|c| CallTarget { key: c.key, label: c.label.clone() }).collect();
            if let Some(error) = &audio.last_error { view.status = error.clone(); }
        }
        if let Some(capture) = capture {
            view.status = match capture.state {
                crate::call_audio::CapturePhase::Starting => "Avvio ascolto…",
                crate::call_audio::CapturePhase::Listening => "Ascolto attivo · Stop interrompe la sessione",
                crate::call_audio::CapturePhase::Stopping => "Arresto ascolto…",
            }.into();
            view.scope = format!("{} · audio dell'app e processi figli; può includere più schede.", capture.label);
            view.provider = if self.call_session.as_ref().is_some_and(|s| s.ai_enabled()) {
                "Whisper locale · testo leggibile da una chat AI attiva".into()
            } else { "Solo trascrizione locale · collegamento AI spento".into() };
        }
        if let Some(notice) = &self.call_notice { view.status.push_str(" · "); view.status.push_str(notice); }
        view.transcript = if self.call_transcript.is_empty() { "La trascrizione comparirà qui dopo l'avvio e una frase pronunciata.".into() } else { self.call_transcript.clone() };
        view.suggestion = if self.call_suggestion.is_empty() {
            if self.call_session.as_ref().is_some_and(|s| s.ai_enabled()) {
                "Ponte testo pronto. Scrivi «avvia assistenza call» nella chat Codex da usare: l'agente leggerà le frasi e pubblicherà qui i suggerimenti mentre quella chat è attiva.".into()
            } else { "Modalità solo trascrizione. Per i suggerimenti termina questa sessione e scegli Trascrivi + AI.".into() }
        } else { self.call_suggestion.clone() };
        view
    }

    fn start_call(&mut self, key: u64, context: String, with_ai: bool) -> Result<(), String> {
        if self.call_session.is_some() { return Err("Ascolto già in corso".into()); }
        if context.len() > 2048 { return Err("Obiettivo della call troppo lungo".into()); }
        let config = crate::call_stt::Configuration::discover()?;
        let monitor = self.monitor.as_ref().ok_or("Rilevamento sessioni audio non disponibile")?;
        monitor.confirm(key)?;
        let capture = match monitor.snapshot().capture {
            Some(capture) => capture,
            None => { monitor.stop_capture(); self.audio = Some(monitor.snapshot()); return Err("Conferma audio non disponibile".into()); }
        };
        let session = match crate::call_session::CallSession::new(capture.generation, config, context, with_ai) {
            Ok(session) => session,
            Err(error) => { monitor.stop_capture(); self.audio = Some(monitor.snapshot()); return Err(error); }
        };
        self.call_session = Some(session);
        self.audio = Some(monitor.snapshot());
        self.call_transcript.clear(); self.call_suggestion.clear(); self.call_notice = None;
        self.manual_call = true;
        Ok(())
    }

    fn tick_call(&mut self) {
        let Some(session) = &mut self.call_session else { return; };
        let active = self.audio.as_ref().and_then(|s| s.capture.as_ref()).is_some_and(|s|
            s.generation == session.generation && s.state != crate::call_audio::CapturePhase::Stopping);
        if !active { self.stop_call(); return; }
        if let Some(monitor) = &self.monitor {
            for chunk in monitor.poll_audio() {
                if let Err(error) = session.push_audio(chunk) { self.call_notice = Some(error); self.stop_call(); return; }
            }
        }
        let result = session.tick();
        self.call_transcript = session.transcript.clone(); self.call_suggestion = session.suggestion.clone();
        if let Some(notice) = &session.notice { self.call_notice = Some(notice.clone()); }
        if let Err(error) = result { self.call_notice = Some(error); self.stop_call(); }
    }

    fn stop_call(&mut self) {
        // Revoke audio first, before waiting for IPC/STT cleanup.
        if let Some(monitor) = &self.monitor { monitor.stop_capture(); }
        if let Some(mut session) = self.call_session.take() {
            session.stop(); self.call_transcript = std::mem::take(&mut session.transcript);
            self.call_suggestion = std::mem::take(&mut session.suggestion);
        }
    }

    fn render(&mut self) {
        if self.availability != OverlayAvailability::Ready { return; }
        if self.config.codex_dashboard { let dashboard = self.dashboard(); self.overlays.apply_dashboard(dashboard); }
        if self.config.call_panel { let call = self.call_view(); self.overlays.apply_call(call); }
    }
}

impl Drop for Assistant {
    fn drop(&mut self) {
        self.stop_call();
        self.server_start = None;
        self.bridge = None; self.metrics = None; self.monitor = None;
        self.overlays.shutdown();
    }
}

fn thread_uri(id: &str) -> Option<String> {
    if id.len() != 36 || !id.bytes().enumerate().all(|(i, b)| {
        if [8, 13, 18, 23].contains(&i) { b == b'-' } else { b.is_ascii_hexdigit() }
    }) { return None; }
    Some(format!("codex://threads/{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn thread(status: Status, message: &str, completed: bool) -> codex_bridge::ThreadView {
        codex_bridge::ThreadView { id: "synthetic".into(), title: "Test".into(), status, last_message: message.into(), questions: vec![], approvals: vec![], token_usage: None, completed, subscription_eligible: true, subscription_ready: true, subscription_error: None }
    }
    #[test]
    fn initial_inventory_is_not_a_fake_new_message_or_completion() {
        let mut attention = Attention::default(); let now = Instant::now();
        update_attention(&mut attention, &thread(Status::Idle, "old", true), false, now);
        assert!(!attention.unread); assert!(attention.until.is_none());
    }
    #[test]
    fn new_message_completion_and_pending_question_raise_attention() {
        let mut attention = Attention::default(); let now = Instant::now();
        update_attention(&mut attention, &thread(Status::Active, "a", false), false, now);
        update_attention(&mut attention, &thread(Status::Idle, "b", true), false, now);
        assert!(attention.unread); assert!(attention.until.unwrap() > now);
        update_attention(&mut attention, &thread(Status::WaitingInput, "b", true), true, now);
        assert!(!attention.unread); assert!(attention.until.unwrap() > now);
    }
    #[test]
    fn panels_require_explicit_opt_in_and_call_audio_stays_off() {
        let map = BTreeMap::new(); assert!(!enabled(&map, DASHBOARD_KEY)); assert!(!enabled(&map, CALL_KEY));
        let assistant = Assistant::new(); let call = assistant.call_view();
        assert!(!call.visible); assert!(!call.can_listen); assert!(!call.listening);
        assert!(assistant.bridge.is_none()); assert!(assistant.monitor.is_none());
    }
    #[test]
    fn every_capture_phase_keeps_the_indicator_visible_even_without_candidates() {
        let mut assistant = Assistant::new(); assistant.config.call_panel = true;
        assistant.manual_call = false;
        for phase in [crate::call_audio::CapturePhase::Starting, crate::call_audio::CapturePhase::Listening, crate::call_audio::CapturePhase::Stopping] {
            assistant.audio = Some(AudioSnapshot { capture: Some(crate::call_audio::CaptureStatus {
                generation: 1, key: 2, label: "Synthetic call".into(), pid: 42, state: phase,
            }), ..Default::default() });
            let view = assistant.call_view();
            assert!(view.visible); assert!(view.listening); assert!(!view.can_listen);
        }
    }
    #[test]
    fn stop_retains_the_transcript_panel_and_close_hides_it() {
        let mut assistant = Assistant::new(); assistant.config.call_panel = true;
        assistant.call_transcript = "Synthetic transcript".into();
        assistant.action(OverlayAction::StopCall).unwrap();
        let view = assistant.call_view(); assert!(view.visible); assert!(!view.listening);
        assert_eq!(view.transcript,"Synthetic transcript");
        assistant.action(OverlayAction::CloseCall).unwrap(); assert!(!assistant.call_view().visible);
    }
    #[test]
    fn only_exact_thread_identifiers_become_deep_links() {
        assert!(thread_uri("00000000-0000-4000-8000-000000000001").is_some());
        for bad in ["", "a?prompt=run", "../thread", "00000000-0000-4000-8000-000000000001?x=1", "zzzzzzzz-0000-4000-8000-000000000001"] {
            assert!(thread_uri(bad).is_none());
        }
    }

    #[test]
    fn question_explanations_remain_readable_and_secret_forms_stay_in_codex() {
        let mut assistant = Assistant::new();
        let mut current = thread(Status::WaitingInput, "Context", false);
        current.questions.push(codex_bridge::QuestionRequest {
            key: codex_bridge::AnswerKey { epoch: 1, request_id: codex_bridge::RequestId::Number(42), thread_id: "synthetic".into(), turn_id: "turn".into() },
            item_id: "item".into(), is_blocking: true,
            questions: vec![codex_bridge::Question { id: "choice".into(), header: "Choice".into(), text: "Choose an option".into(),
                options: vec![codex_bridge::QuestionOption { label: "A".into(), description: "Explanation of A".into() }], is_secret: false, allows_other: false }],
        });
        assistant.selected_thread = Some("synthetic".into()); assistant.codex.threads.push(current);
        let view = assistant.dashboard();
        assert!(view.detail.contains("A: Explanation of A"));
        assert_eq!(view.question.unwrap().questions[0].options, ["A"]);
        assistant.codex.threads[0].questions[0].questions[0].is_secret = true;
        let view = assistant.dashboard(); assert!(view.question.is_none());
        assert!(view.approval_notice.unwrap().contains("riservato"));
    }
}
