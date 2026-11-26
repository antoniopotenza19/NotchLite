// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Explicit headless checks. Normal startup never creates chats or sends prompts.
use std::path::PathBuf;
use std::time::{Duration, Instant};
use crate::codex_bridge::{AccountState, Bridge, Connection, Snapshot};

fn connect(cli: &Option<PathBuf>) -> Result<Bridge, String> {
    match cli { Some(path) => Bridge::start_with_cli(None, path.clone()), None => Bridge::start(None) }
}
fn wait(bridge: &Bridge, seconds: u64, predicate: impl Fn(&Snapshot) -> bool) -> Result<Snapshot, String> {
    let until = Instant::now() + Duration::from_secs(seconds);
    loop {
        let state = bridge.snapshot();
        if let Connection::Unavailable(reason) = &state.connection { return Err(reason.clone()); }
        if let AccountState::Unavailable(reason) = &state.account { return Err(reason.clone()); }
        if predicate(&state) { return Ok(state); }
        if Instant::now() >= until {
            eprintln!("{}", serde_json::json!({"diagnostic":"timeout","threads":state.threads.len(),
                "states":state.threads.iter().map(|t|format!("{:?}",t.status)).collect::<Vec<_>>(),
                "completions":state.threads.iter().filter(|t|t.completed).count(),
                "responseMarkers":state.threads.iter().filter(|t|t.last_message.contains("NOTCHLITE_OK")).count(),
                "questions":state.threads.iter().map(|t|t.questions.len()).sum::<usize>(),
                "approvals":state.threads.iter().map(|t|t.approvals.len()).sum::<usize>()}));
            if let Some(error) = &state.last_rpc_error {
                eprintln!("{}", serde_json::json!({"rpcMethod":error.method,"code":error.code,"reason":error.reason}));
            }
            return Err(format!("Verifica Codex scaduta{}", state.last_error.map(|e| format!(". Ultimo stato: {e}")).unwrap_or_default()));
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

/// Called before profile, COM, mutex or window initialization.
pub fn run() -> Result<bool, String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let modes: Vec<_> = ["--codex-server-start", "--codex-connect-check", "--codex-self-test"]
        .into_iter().filter(|mode| args.iter().any(|a| a == *mode)).collect();
    if modes.is_empty() { return Ok(false); }
    if modes.len() != 1 { return Err("Scegli una sola verifica Codex per volta".into()); }
    let mut cli = None;
    let mut workspace = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == modes[0] { index += 1; continue; }
        let destination = if args[index] == "--codex-cli" { &mut cli }
            else if args[index] == "--workspace" { &mut workspace }
            else { return Err("Argomento della verifica Codex non riconosciuto".into()); };
        if destination.is_some() { return Err("Argomento duplicato nella verifica Codex".into()); }
        *destination = Some(PathBuf::from(args.get(index + 1).ok_or("Manca il percorso dopo l'opzione Codex")?));
        index += 2;
    }
    if modes[0] == "--codex-server-start" {
        let worker = crate::codex_server::StartWorker::start(cli)?;
        let until = Instant::now() + Duration::from_secs(23);
        loop {
            if let Some(result) = worker.poll() {
                result?;
                println!("{}", serde_json::json!({"sharedServerAvailable":true,"guiOpened":false}));
                return Ok(true);
            }
            if Instant::now() >= until { return Err("Avvio del server locale scaduto".into()); }
            std::thread::sleep(Duration::from_millis(40));
        }
    }
    let client = connect(&cli)?;
    wait(&client, 15, |s| s.connection == Connection::Connected && s.account.is_ready())?;
    if modes[0] == "--codex-connect-check" {
        // This check reads metadata; it never starts a turn or submits a message.
        let state = match wait(&client, 10, |s| s.quota.is_some()) {
            Ok(state) => state,
            Err(reason) => {
                let latest = client.snapshot();
                if latest.connection != Connection::Connected || !latest.account.is_ready() { return Err(reason); }
                latest
            }
        };
        println!("{}", serde_json::json!({"connected":true,"chatGptAccount":true,
            "loadedThreads":state.threads.len(),"quotaAvailable":state.quota.is_some(),"guiOpened":false}));
        return Ok(true);
    }
    // Explicit smoke test: one new thread, a materializing turn and a shared turn.
    let workspace = workspace.ok_or("--codex-self-test richiede --workspace con una cartella di prova esplicita")?;
    let workspace = workspace.to_str().ok_or("Percorso della cartella non rappresentabile")?;
    client.create_thread(workspace)?;
    let state = wait(&client, 15, |s| s.created_thread_id.is_some() && !s.creating_thread)?;
    eprintln!("{{\"diagnostic\":\"thread-created\"}}");
    let id = state.created_thread_id.as_ref().ok_or("ID della chat di prova non disponibile")?.clone();
    // A new empty thread may not yet have a resumable rollout for another client.
    // Materialize one turn through the creator before testing cross-client events.
    client.follow_up(&id, "Rispondi solo NOTCHLITE_READY. Non usare strumenti e non leggere o modificare file.")?;
    wait(&client, 90, |s| s.threads.iter().any(|t| t.id == id && t.completed && t.last_message.contains("NOTCHLITE_READY")))?;
    eprintln!("{{\"diagnostic\":\"first-turn-persisted\"}}");
    let observer = connect(&cli)?;
    wait(&observer, 15, |s| s.account.is_ready() && s.threads.iter().any(|t| t.id == id && t.subscription_ready && t.completed))?;
    eprintln!("{{\"diagnostic\":\"second-client-subscribed-and-history-recovered\"}}");
    // No tools, files, credentials or other conversations are part of the prompt.
    client.follow_up(&id, "Rispondi solo NOTCHLITE_OK. Non usare strumenti e non leggere o modificare file.")?;
    eprintln!("{{\"diagnostic\":\"test-message-requested\"}}");
    let produced = wait(&client, 90, |s| s.threads.iter().any(|t| t.id == id && t.completed && t.last_message.contains("NOTCHLITE_OK")))?;
    eprintln!("{{\"diagnostic\":\"first-client-completed\"}}");
    let observed = wait(&observer, 15, |s| s.threads.iter().any(|t| t.id == id && t.completed && t.last_message.contains("NOTCHLITE_OK")))?;
    let a = produced.threads.iter().find(|t| t.id == id).unwrap();
    let b = observed.threads.iter().find(|t| t.id == id).unwrap();
    println!("{}", serde_json::json!({"connected":true,"chatGptAccount":true,"clients":2,
        "sameThreadObserved":true,"responseMarkerObserved":true,"completionObserved":true,
        "tokenUsageObservedByBoth":a.token_usage.is_some() && b.token_usage.is_some(),
        "inputTokens":a.token_usage.as_ref().map(|u|u.input),"outputTokens":a.token_usage.as_ref().map(|u|u.output),
        "testThreadCreated":true,"messagesSent":2,"historyRecoveryVerified":true,"guiOpened":false}));
    Ok(true)
}
