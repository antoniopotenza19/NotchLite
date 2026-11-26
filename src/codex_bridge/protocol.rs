// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
use super::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) const MAX_THREADS: usize = 32;
const MAX_PREVIEW: usize = 2048;

pub(super) fn short(value: &str, max: usize) -> String {
    let mut end = value.len().min(max);
    while !value.is_char_boundary(end) { end -= 1; }
    value[..end].to_owned()
}
fn text(value: &Value, key: &str, max: usize) -> String { short(value[key].as_str().unwrap_or(""), max) }
fn identity(value: &Value) -> Option<RequestId> {
    value.as_i64().map(RequestId::Number).or_else(|| value.as_str().filter(|s| s.len() <= 128).map(|s| RequestId::Text(s.into())))
}
fn wire_id(id: &RequestId) -> Value { match id { RequestId::Number(n) => json!(n), RequestId::Text(s) => json!(s) } }
fn status(value: &Value) -> Status {
    match value["type"].as_str() {
        Some("idle") => Status::Idle,
        Some("active") => {
            let flags = value["activeFlags"].as_array();
            if flags.is_some_and(|f| f.iter().any(|v| v == "waitingOnUserInput")) { Status::WaitingInput }
            else if flags.is_some_and(|f| f.iter().any(|v| v == "waitingOnApproval")) { Status::WaitingApproval }
            else { Status::Active }
        }
        _ => Status::Unavailable,
    }
}
#[derive(Clone)]
enum Pending { Initialize, Account, CreateThread, Loaded(u64), Quota, Read(String), Resume(String), History(String,u64,u8), Enqueue(String), StartQueued(String) }
#[derive(Clone)]
enum Prompt { Question(QuestionRequest), Approval(Approval) }

pub(super) struct State {
    pub snapshot: Snapshot,
    pub outbox: VecDeque<Value>,
    next: u64,
    pending: BTreeMap<String, Pending>,
    prompts: BTreeMap<RequestId, Prompt>,
    loaded: BTreeSet<String>,
    attached: BTreeSet<String>,
    restricted: BTreeSet<String>,
    topology_revision: u64,
    event_clock: u64,
    event_versions: BTreeMap<String,u64>,
}
impl State {
    pub fn new(snapshot: Snapshot) -> Self {
        Self { snapshot, outbox: VecDeque::new(), next: 0, pending: BTreeMap::new(), prompts: BTreeMap::new(), loaded: BTreeSet::new(), attached: BTreeSet::new(), restricted: BTreeSet::new(),topology_revision:0,event_clock:0,event_versions:BTreeMap::new() }
    }
    pub fn begin(&mut self) {
        self.request("initialize", json!({"clientInfo":{"name":"notchlite_dashboard","title":"NotchLite Dashboard","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true,"requiredHumanApproval":true}}), Pending::Initialize);
    }
    fn request(&mut self, method: &str, params: Value, pending: Pending) -> bool {
        if self.pending.len() >= 128 || self.outbox.len() >= 128 { self.error("Troppe richieste Codex: aggiorna il pannello"); return false; }
        self.next = self.next.saturating_add(1);
        let id = format!("notchlite-{}-{}", self.snapshot.epoch, self.next);
        self.pending.insert(id.clone(), pending);
        self.outbox.push_back(json!({"id":id,"method":method,"params":params}));
        true
    }
    fn read_account(&mut self) {
        if !self.pending.values().any(|p|matches!(p,Pending::Account)) {
            self.request("account/read",json!({"refreshToken":false}),Pending::Account);
        }
    }
    pub fn refresh(&mut self, quota: bool) {
        if self.snapshot.connection != Connection::Connected || !self.snapshot.account.is_ready() { return; }
        if !self.pending.values().any(|p| matches!(p, Pending::Loaded(_))) {
            self.request("thread/loaded/list", json!({"limit":MAX_THREADS}), Pending::Loaded(self.topology_revision));
        }
        if quota && !self.pending.values().any(|p| matches!(p, Pending::Quota)) {
            self.request("account/rateLimits/read", json!({}), Pending::Quota);
        }
    }
    pub fn error(&mut self, message: &str) {
        self.snapshot.last_error = Some(short(message, 200)); self.changed();
    }
    fn changed(&mut self) { self.snapshot.revision = self.snapshot.revision.wrapping_add(1); }
    pub fn message(&mut self, message: Value) {
        if let Some(method) = message["method"].as_str() {
            if message.get("id").is_some() { self.prompt(method, &message); }
            else { self.notification(method, &message["params"]); }
            return;
        }
        let Some(id) = message["id"].as_str() else { return; };
        let Some(pending) = self.pending.remove(id) else { return; };
        if matches!(pending,Pending::CreateThread){self.snapshot.creating_thread=false;}
        if message.get("error").is_some() {
            let diagnostic=rpc_error(&pending,&message["error"]);
            self.snapshot.last_rpc_error=Some(diagnostic.clone());
            if let Pending::Resume(id)=&pending {
                if let Some(thread)=self.thread(id){thread.subscription_ready=false;thread.subscription_error=Some(diagnostic);}
            }
            let note = match pending {
                Pending::Account => {
                    self.snapshot.account=AccountState::Unavailable("Accesso ChatGPT non verificabile".into());
                    "Impossibile verificare l'abbonamento ChatGPT in Codex."
                }
                Pending::CreateThread => "Chat non creata: verifica il workspace e le impostazioni in Codex.",
                Pending::Enqueue(_) => "Invio non disponibile su questa versione Codex. Apri Codex per continuare.",
                Pending::StartQueued(_) => "Messaggio in coda sul PC; non viene reinviato automaticamente.",
                Pending::Resume(ref id) => {
                    self.attached.remove(id);
                    "Una chat non è collegabile dal pannello: aprila in Codex."
                }
                Pending::Quota => "Quota non disponibile dal provider Codex.",
                _ => "Richiesta Codex non completata. Aggiorna il pannello o apri Codex.",
            };
            self.error(note); return;
        }
        let result = &message["result"];
        match pending {
            Pending::Initialize => {
                self.snapshot.connection = Connection::Connected;
                self.snapshot.server_info = text(result, "userAgent", 160);
                self.outbox.push_back(json!({"method":"initialized","params":{}})); self.read_account();
            }
            Pending::Account => {
                if result["account"]["type"]=="chatgpt" {
                    self.snapshot.account=AccountState::ChatGpt{plan:text(&result["account"],"planType",64)};
                    self.snapshot.last_error=None;
                    self.refresh(true);
                } else {
                    let reason="È richiesto l'accesso ChatGPT in Codex: le API key e altri provider non sono usati dal pannello.";
                    self.snapshot.account=AccountState::Unavailable(reason.into());
                    self.snapshot.threads.clear();self.loaded.clear();self.attached.clear();self.prompts.clear();
                    self.error(reason);
                }
            }
            Pending::CreateThread => {
                let Some(id)=result["thread"]["id"].as_str().filter(|s|!s.is_empty()&&s.len()<=128).map(str::to_owned) else {
                    self.error("Codex non ha restituito un ID valido per la nuova chat.");return;
                };
                let policies_ok=result["sandbox"]["type"]=="readOnly" && result["approvalPolicy"]=="untrusted" && result["approvalsReviewer"]=="user" && result["modelProvider"]=="openai";
                if !policies_ok{self.restricted.insert(id.clone());}
                self.loaded.insert(id.clone());self.attached.insert(id.clone());
                self.topology_revision=self.topology_revision.wrapping_add(1);
                self.metadata(&id,&result["thread"]);
                self.snapshot.created_thread_id=Some(id.clone());
                // Creating a thread subscribes this connection, without starting a user turn.
                if let Some(thread)=self.thread(&id) {
                    thread.subscription_eligible&=policies_ok;
                    if thread.title=="Chat Codex" {thread.title=if policies_ok{"Nuova chat · sola lettura"}else{"Nuova chat · verifica policy"}.into();}
                }
                if policies_ok{self.snapshot.last_error=Some("Chat creata in sola lettura. Nessun messaggio inviato.".into());}
                else{self.error("Chat creata con impostazioni diverse da quelle richieste: continua nell'app Codex.");}
            }
            Pending::Loaded(revision) => {
                // The response predates a create/close ACK and is not an authoritative replacement.
                if revision!=self.topology_revision{return;}
                let Some(data) = result["data"].as_array() else { self.error("Risposta Codex non valida"); return; };
                let ids: BTreeSet<String> = data.iter().filter_map(Value::as_str).filter(|s| !s.is_empty() && s.len() <= 128).take(MAX_THREADS).map(str::to_owned).collect();
                self.loaded = ids.clone();
                self.cancel_unloaded_followups();
                self.snapshot.threads.retain(|t| ids.contains(&t.id));
                self.attached.retain(|id| ids.contains(id));
                self.event_versions.retain(|id,_|ids.contains(id));
                self.prompts.retain(|_, p| ids.contains(&prompt_key(p).thread_id));
                for id in ids { self.request("thread/read", json!({"threadId":id,"includeTurns":false}), Pending::Read(id)); }
                if result["nextCursor"].is_string() { self.error("Mostrate al massimo 32 chat caricate sul server collegato."); }
            }
            Pending::Read(id) => {
                if !self.loaded.contains(&id) { return; }
                self.metadata(&id, &result["thread"]);
                if !self.attached.contains(&id) && !self.pending.values().any(|p|matches!(p,Pending::Resume(thread) if thread==&id)) {
                    // Only IDs returned as already loaded are resumed, with no permission/config override.
                    if let Some(thread)=self.thread(&id){thread.subscription_error=None;}
                    self.request("thread/resume", json!({"threadId":id,"excludeTurns":true}), Pending::Resume(id));
                }
            }
            Pending::Resume(id) => {
                if self.loaded.contains(&id) {
                    self.attached.insert(id.clone());
                    self.metadata(&id, &result["thread"]);
                    if let Some(thread)=self.thread(&id){thread.subscription_error=None;}
                    if !self.snapshot.threads.iter().any(|t|t.subscription_error.is_some())
                        && self.snapshot.last_rpc_error.as_ref().is_some_and(|e|e.method=="thread/resume") {
                        self.snapshot.last_rpc_error=None;
                        if self.snapshot.last_error.as_deref()==Some("Una chat non è collegabile dal pannello: aprila in Codex.") {self.snapshot.last_error=None;}
                    }
                    self.request_history(id,0);
                }
            }
            Pending::History(id,revision,retries) => {
                if !self.loaded.contains(&id){return;}
                if self.event_versions.get(&id).copied().unwrap_or(0)!=revision {
                    // A late Idle notification can invalidate a useful snapshot. Retry only
                    // this local history read, never a message, and bound the race recovery.
                    let needs_history=self.snapshot.threads.iter().any(|t|t.id==id&&t.status==Status::Idle&&(!t.completed||t.last_message.is_empty()));
                    if needs_history&&retries<2&&!self.pending.values().any(|p|matches!(p,Pending::History(thread,_,_) if thread==&id)){self.request_history(id,retries+1);}
                    return;
                }
                if let Some(items) = result["data"][0]["items"].as_array() {
                    if let Some(item) = items.iter().rev().find(|i| i["type"] == "agentMessage") {
                        if let Some(thread) = self.thread(&id) { if thread.last_message.is_empty() { thread.last_message = text(item,"text",MAX_PREVIEW); } }
                    }
                }
                if let Some(thread)=self.thread(&id) {
                    if thread.status==Status::Idle && thread.questions.is_empty() && thread.approvals.is_empty() {
                        match result["data"][0]["status"].as_str(){
                            Some("completed")=>thread.completed=true,
                            Some("failed"|"interrupted")=>thread.completed=false,
                            _=>{},
                        }
                    }
                }
            }
            Pending::Quota => { self.snapshot.quota = Some(parse_quota(result)); }
            Pending::Enqueue(thread) => {
                let id = text(&result["queuedSubmission"], "id", 128);
                if id.is_empty() { self.error("Il PC non ha confermato il messaggio in coda."); }
                else {
                    self.snapshot.last_error = Some("Messaggio in coda sul PC".into());
                    self.request("thread/queue/start", json!({"threadId":thread,"queuedSubmissionId":id}), Pending::StartQueued(thread));
                }
            }
            Pending::StartQueued(_) => {}
        }
        self.changed();
    }
    fn thread(&mut self, id: &str) -> Option<&mut ThreadView> { self.snapshot.threads.iter_mut().find(|t| t.id == id) }
    fn request_history(&mut self,id:String,retries:u8){
        let revision=self.event_versions.get(&id).copied().unwrap_or(0);
        self.request("thread/turns/list",json!({"threadId":id,"limit":1,"sortDirection":"desc","itemsView":"full"}),Pending::History(id,revision,retries));
    }
    fn metadata(&mut self, id: &str, raw: &Value) {
        if raw["id"].as_str() != Some(id) { return; }
        if self.thread(id).is_none() && self.snapshot.threads.len() < MAX_THREADS {
            self.bump_event_version(id);
            self.snapshot.threads.push(ThreadView { id:id.into(),title:String::new(),status:Status::Unavailable,last_message:String::new(),subscription_eligible:false,subscription_ready:false,subscription_error:None,questions:Vec::new(),approvals:Vec::new(),token_usage:None,completed:false });
        }
        let restricted=self.restricted.contains(id);
        let attached=self.attached.contains(id);
        if let Some(thread) = self.thread(id) {
            thread.title = raw["name"].as_str().filter(|s| !s.is_empty()).map(|s| short(s,120)).unwrap_or_else(|| "Chat Codex".into());
            thread.status = status(&raw["status"]);
            thread.subscription_eligible=raw["modelProvider"]=="openai"&&!restricted;
            thread.subscription_ready=attached;
        }
    }
    fn notification(&mut self, method: &str, p: &Value) {
        if method=="account/updated" {
            if self.snapshot.connection!=Connection::Connected{return;}
            self.snapshot.epoch=next_epoch();self.snapshot.account=AccountState::Checking;self.snapshot.creating_thread=false;self.snapshot.created_thread_id=None;
            self.snapshot.threads.clear();self.snapshot.quota=None;self.loaded.clear();self.attached.clear();self.prompts.clear();
            self.topology_revision=self.topology_revision.wrapping_add(1);self.event_versions.clear();
            self.pending.clear();self.outbox.clear();self.read_account();self.changed();return;
        }
        if !self.snapshot.account.is_ready(){return;}
        let id = text(p,"threadId",128);
        if self.loaded.contains(&id) && matches!(method,"thread/status/changed"|"turn/started"|"turn/completed"|"item/agentMessage/delta"|"item/completed") {self.bump_event_version(&id);}
        match method {
            "thread/status/changed" => if let Some(t) = self.thread(&id) { t.status = status(&p["status"]); },
            "turn/started" => if let Some(t) = self.thread(&id) { t.status=Status::Active; t.completed=false; t.last_message.clear(); },
            "turn/completed" => {
                if let Some(t) = self.thread(&id) { t.status=Status::Idle; t.completed=p["turn"]["status"]=="completed"; }
                let turn = p["turn"]["id"].as_str().unwrap_or("");
                self.prompts.retain(|_,v| !(prompt_key(v).thread_id==id && prompt_key(v).turn_id==turn));
                self.sync_prompts();
            }
            "item/agentMessage/delta" => if let Some(t) = self.thread(&id) {
                if let Some(delta) = p["delta"].as_str() {
                    // Keep the most recent UTF-8 tail, rather than unbounded streamed history.
                    let mut tail = std::mem::take(&mut t.last_message); tail.push_str(&short(delta,MAX_PREVIEW));
                    let mut start = tail.len().saturating_sub(MAX_PREVIEW); while !tail.is_char_boundary(start) { start+=1; }
                    t.last_message=tail[start..].into();
                }
            },
            "item/completed" if p["item"]["type"]=="agentMessage" => if let Some(t)=self.thread(&id) { t.last_message=text(&p["item"],"text",MAX_PREVIEW); },
            "thread/tokenUsage/updated" => if let Some(t)=self.thread(&id) {
                let u=&p["tokenUsage"]; let n=&u["total"];
                t.token_usage=Some(TokenUsage { input:n["inputTokens"].as_i64().unwrap_or(0),cached_input:n["cachedInputTokens"].as_i64().unwrap_or(0),output:n["outputTokens"].as_i64().unwrap_or(0),reasoning_output:n["reasoningOutputTokens"].as_i64().unwrap_or(0),total:n["totalTokens"].as_i64().unwrap_or(0),context_window:u["modelContextWindow"].as_i64() });
            },
            "account/rateLimits/updated" => self.snapshot.quota=Some(parse_quota(p)),
            "serverRequest/resolved" => { if let Some(id)=identity(&p["requestId"]) { self.prompts.remove(&id); self.sync_prompts(); } },
            "thread/closed" => {
                self.topology_revision=self.topology_revision.wrapping_add(1);self.event_versions.remove(&id);
                self.loaded.remove(&id); self.attached.remove(&id); self.snapshot.threads.retain(|t| t.id!=id);
                self.cancel_unloaded_followups();
                self.prompts.retain(|_,v|prompt_key(v).thread_id!=id);
            }
            _ => return,
        }
        self.changed();
    }
    fn prompt(&mut self, method: &str, message: &Value) {
        if !self.snapshot.account.is_ready(){return;}
        let Some(id)=identity(&message["id"]) else{return}; let p=&message["params"];
        if ["threadId","turnId"].iter().any(|field| !p[*field].as_str().is_some_and(|s|!s.is_empty()&&s.len()<=128)){return;}
        let key=AnswerKey{epoch:self.snapshot.epoch,request_id:id.clone(),thread_id:text(p,"threadId",128),turn_id:text(p,"turnId",128)};
        if !self.loaded.contains(&key.thread_id) || self.prompts.len()>=32 {return;}
        let prompt=match method {
            "item/tool/requestUserInput" => {
                let Some(raw)=p["questions"].as_array().filter(|a|!a.is_empty()&&a.len()<=3) else {self.error("Domanda non visualizzabile: apri Codex.");return;};
                let mut questions=Vec::new();let mut seen=BTreeSet::new();
                for q in raw {
                    let Some(qid)=q["id"].as_str().filter(|s|!s.is_empty()&&s.len()<=128) else{return;};
                    if !seen.insert(qid){return;}
                    let options=q["options"].as_array(); if options.is_some_and(|a|a.len()>32){return;}
                    if options.is_some_and(|a|a.iter().any(|o|!o["label"].as_str().is_some_and(|s|s.len()<=256))){self.error("Opzioni troppo lunghe: rispondi in Codex.");return;}
                    let options=options.into_iter().flatten().map(|o|QuestionOption{label:text(o,"label",256),description:text(o,"description",512)}).collect();
                    questions.push(Question{id:qid.into(),header:text(q,"header",80),text:text(q,"question",2048),options,is_secret:q["isSecret"].as_bool().unwrap_or(false),allows_other:q["isOther"].as_bool().unwrap_or(false)});
                }
                Prompt::Question(QuestionRequest{key,item_id:text(p,"itemId",128),questions,is_blocking:p["isBlocking"].as_bool().unwrap_or(true)})
            }
            "item/commandExecution/requestApproval"|"item/fileChange/requestApproval"|"item/permissions/requestApproval" => {
                let label=p["reason"].as_str().or(p["command"].as_str()).unwrap_or("Richiesta di permesso: apri Codex per approvare");
                Prompt::Approval(Approval{key,kind:method.into(),label:short(label,1024),can_deny:method!="item/permissions/requestApproval"})
            }
            _=>return, // Never execute tools or reply automatically to another client's RPCs.
        };
        self.prompts.insert(id,prompt);self.sync_prompts();self.changed();
    }
    fn sync_prompts(&mut self) {
        for t in &mut self.snapshot.threads {t.questions.clear();t.approvals.clear();}
        let values:Vec<_>=self.prompts.values().cloned().collect();
        for p in values {if let Some(t)=self.thread(&prompt_key(&p).thread_id){match p{Prompt::Question(q)=>t.questions.push(q),Prompt::Approval(a)=>t.approvals.push(a)}}}
    }
    fn bump_event_version(&mut self,id:&str){self.event_clock=self.event_clock.wrapping_add(1);self.event_versions.insert(id.into(),self.event_clock);}
    fn cancel_unloaded_followups(&mut self) {
        let removed: BTreeSet<_> = self.pending.iter().filter_map(|(id, pending)| {
            match pending {
                Pending::Enqueue(thread) | Pending::StartQueued(thread) if !self.loaded.contains(thread) => Some(id.clone()),
                _ => None,
            }
        }).collect();
        self.pending.retain(|id, _| !removed.contains(id));
        self.outbox.retain(|message| !message["id"].as_str().is_some_and(|id| removed.contains(id)));
    }
    pub fn command(&mut self, command: Command) {
        if !matches!(&command,Command::Refresh) && (self.snapshot.connection!=Connection::Connected||!self.snapshot.account.is_ready()) {
            self.error("Collega un account ChatGPT in Codex prima di inviare azioni.");return;
        }
        let target=match &command{Command::Answer(key,_)|Command::Deny(key)=>Some(&key.thread_id),Command::FollowUp(id,_)=>Some(id),_=>None};
        if target.is_some_and(|id|!self.snapshot.threads.iter().any(|t|&t.id==id&&t.subscription_eligible)) {
            self.error("Questa chat non usa il provider OpenAI con l'accesso ChatGPT del pannello.");return;
        }
        match command {
            Command::Refresh=>{if self.snapshot.account.is_ready(){self.refresh(true);}else if self.snapshot.connection==Connection::Connected{self.read_account();}},
            Command::CreateThread(cwd)=>{
                if self.snapshot.creating_thread||self.pending.values().any(|p|matches!(p,Pending::CreateThread)){self.error("Creazione chat già in corso");return;}
                if self.snapshot.threads.len()>=MAX_THREADS{self.error("Limite di 32 chat caricate nel pannello");return;}
                if self.restricted.len()>=MAX_THREADS{self.error("Verifica le policy delle chat in Codex prima di crearne altre.");return;}
                self.snapshot.created_thread_id=None;
                if self.request("thread/start",json!({"cwd":cwd,"sandbox":"read-only","approvalPolicy":"untrusted","approvalsReviewer":"user","modelProvider":"openai"}),Pending::CreateThread){self.snapshot.creating_thread=true;self.snapshot.last_error=None;}
            }
            Command::Answer(key,answers)=>{
                let Some(Prompt::Question(q))=self.prompts.get(&key.request_id) else{self.error("Domanda già risolta o scaduta");return;};
                if q.key!=key || key.epoch!=self.snapshot.epoch || answers.len()!=q.questions.len() || q.questions.iter().any(|q|!answers.contains_key(&q.id)) {self.error("Risposta non associata alla domanda attuale");return;}
                let map:serde_json::Map<String,Value>=answers.into_iter().map(|(id,answers)|(id,json!({"answers":answers}))).collect();
                self.outbox.push_back(json!({"id":wire_id(&key.request_id),"result":{"answers":map}}));
                self.prompts.remove(&key.request_id);self.sync_prompts();
            }
            Command::Deny(key)=>{
                let Some(Prompt::Approval(a))=self.prompts.get(&key.request_id) else{self.error("Richiesta già risolta o scaduta");return;};
                if a.key!=key || !a.can_deny {self.error("Richiesta da gestire in Codex");return;}
                self.outbox.push_back(json!({"id":wire_id(&key.request_id),"result":{"decision":"decline"}}));
                self.prompts.remove(&key.request_id);self.sync_prompts();
            }
            Command::FollowUp(id,input)=>{
                if self.pending.values().any(|pending| matches!(pending, Pending::Enqueue(thread) | Pending::StartQueued(thread) if thread == &id)) {
                    self.error("Invio già in corso"); return;
                }
                if !self.snapshot.threads.iter().any(|t|t.id==id&&t.status==Status::Idle&&t.subscription_ready) || !self.attached.contains(&id) {self.error("La chat non è ancora collegata e inattiva: invio annullato");return;}
                self.next=self.next.saturating_add(1);let nonce=format!("notchlite-{}-{}",self.snapshot.epoch,self.next);
                self.request("thread/queue/add",json!({"threadId":id,"clientUserMessageId":nonce,"input":[{"type":"text","text":input}]}),Pending::Enqueue(id));
            }
        }
        self.changed();
    }
}
fn rpc_error(pending:&Pending,error:&Value)->RpcError {
    let method=match pending{
        Pending::Initialize=>"initialize",Pending::Account=>"account/read",Pending::CreateThread=>"thread/start",
        Pending::Loaded(_)=>"thread/loaded/list",Pending::Quota=>"account/rateLimits/read",Pending::Read(_)=>"thread/read",
        Pending::Resume(_)=>"thread/resume",Pending::History(_,_,_)=>"thread/turns/list",Pending::Enqueue(_)=>"thread/queue/add",Pending::StartQueued(_)=>"thread/queue/start",
    };
    let code=error["code"].as_i64();
    let message=error["message"].as_str().unwrap_or("").to_ascii_lowercase();
    // Never retain the backend message: it can contain paths, identifiers, or credentials.
    let reason=if message.contains("no rollout") {"no_rollout"}
        else if message.contains("not found")||message.contains("not loaded") {"not_found"}
        else if message.contains("account")||message.contains("authenticated owner") {"account_mismatch"}
        else if message.contains("stale path") {"stale_path"}
        else if message.contains("already running")||message.contains("active or pending turn") {"already_running"}
        else if code==Some(-32601)||message.contains("not supported")||message.contains("unknown method") {"unsupported"}
        else if code==Some(-32602) {"invalid_params"}
        else {"other"};
    RpcError{method,code,reason}
}
fn prompt_key(prompt:&Prompt)->&AnswerKey{match prompt{Prompt::Question(q)=>&q.key,Prompt::Approval(a)=>&a.key}}
fn parse_quota(value:&Value)->Quota {
    let mut windows=Vec::new();
    let mut add=|bucket:&str,raw:&Value|{for label in ["primary","secondary"]{let w=&raw[label];if let Some(percent)=w["usedPercent"].as_i64().filter(|n|(0..=100).contains(n)){windows.push(QuotaWindow{bucket:short(bucket,80),label:label.into(),used_percent:percent,resets_at:w["resetsAt"].as_i64(),window_duration_mins:w["windowDurationMins"].as_i64()});}}};
    if let Some(buckets)=value["rateLimitsByLimitId"].as_object(){for (id,v) in buckets.iter().take(8){add(id,v);}}
    else{let raw=&value["rateLimits"];add(raw["limitId"].as_str().unwrap_or("Codex"),raw);}
    Quota{windows}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state()->State {
        let mut s=State::new(Snapshot{epoch:next_epoch(),connection:Connection::Connected,account:AccountState::ChatGpt{plan:"plus".into()},..Snapshot::unavailable("")});
        s.loaded.insert("t".into());s.attached.insert("t".into());s.metadata("t",&json!({"id":"t","name":"Test","modelProvider":"openai","status":{"type":"idle"}}));s
    }
    fn question(s:&mut State){s.message(json!({"id":42,"method":"item/tool/requestUserInput","params":{"threadId":"t","turnId":"turn","itemId":"i","isBlocking":true,"questions":[{"id":"q","header":"Choice","question":"Choose","options":[{"label":"A","description":"A"}]}]}}));}
    #[test]fn stale_epoch_and_resolved_question_cannot_send(){
        let mut s=state();question(&mut s);let key=s.snapshot.threads[0].questions[0].key.clone();let mut stale=key.clone();stale.epoch+=1;
        s.command(Command::Answer(stale,BTreeMap::from([("q".into(),vec!["A".into()])])));assert!(s.outbox.is_empty());
        s.message(json!({"method":"serverRequest/resolved","params":{"threadId":"t","requestId":42}}));
        s.command(Command::Answer(key,BTreeMap::from([("q".into(),vec!["A".into()])])));assert!(s.outbox.is_empty());
    }
    #[test]fn answer_preserves_rpc_identity_and_is_not_a_new_turn(){
        let mut s=state();question(&mut s);let key=s.snapshot.threads[0].questions[0].key.clone();
        s.command(Command::Answer(key.clone(),BTreeMap::from([("q".into(),vec!["A".into()])])));
        assert_eq!(s.outbox.pop_front(),Some(json!({"id":42,"result":{"answers":{"q":{"answers":["A"]}}}})));
        s.command(Command::Answer(key,BTreeMap::from([("q".into(),vec!["A".into()])])));assert!(s.outbox.is_empty());
    }
    #[test]fn followup_uses_only_queue_and_never_steers(){
        let mut s=state();s.command(Command::FollowUp("t".into(),"Next".into()));let add=s.outbox.pop_front().unwrap();assert_eq!(add["method"],"thread/queue/add");
        s.message(json!({"id":add["id"],"result":{"queuedSubmission":{"id":"queued"}}}));let start=s.outbox.pop_front().unwrap();assert_eq!(start["method"],"thread/queue/start");assert_eq!(start["params"]["queuedSubmissionId"],"queued");
        s.snapshot.threads[0].status=Status::Active;s.command(Command::FollowUp("t".into(),"No".into()));assert!(s.outbox.is_empty());
    }
    #[test]fn rapid_followups_are_single_flight_until_final_ack_not_text_deduplicated(){
        let mut s=state();
        s.command(Command::FollowUp("t".into(),"Same message".into()));
        s.command(Command::FollowUp("t".into(),"Same message".into()));
        assert_eq!(s.snapshot.last_error.as_deref(),Some("Invio già in corso"));
        assert_eq!(s.outbox.len(),1);
        let add=s.outbox.pop_front().unwrap();
        s.message(json!({"id":add["id"],"result":{"queuedSubmission":{"id":"queued"}}}));
        let start=s.outbox.pop_front().unwrap();
        s.command(Command::FollowUp("t".into(),"Different message".into()));
        assert!(s.outbox.is_empty());
        s.message(json!({"id":start["id"],"result":{"turn":{"id":"turn","status":"inProgress"}}}));
        s.command(Command::FollowUp("t".into(),"Same message".into()));
        let next=s.outbox.pop_front().unwrap();
        assert_eq!(next["method"],"thread/queue/add");
        assert_ne!(next["params"]["clientUserMessageId"],add["params"]["clientUserMessageId"]);
    }
    #[test]fn failed_or_closed_thread_releases_followup_guard_without_retry(){
        let mut s=state();s.command(Command::FollowUp("t".into(),"First".into()));
        let add=s.outbox.pop_front().unwrap();
        s.message(json!({"id":add["id"],"error":{"code":-32601,"message":"unsupported"}}));
        assert!(s.outbox.is_empty());
        s.command(Command::FollowUp("t".into(),"Second".into()));assert_eq!(s.outbox.len(),1);
        s.message(json!({"method":"thread/closed","params":{"threadId":"t"}}));
        assert!(s.outbox.is_empty());
        assert!(!s.pending.values().any(|p|matches!(p,Pending::Enqueue(_)|Pending::StartQueued(_))));
        s.loaded.insert("t".into());s.attached.insert("t".into());s.metadata("t",&json!({"id":"t","modelProvider":"openai","status":{"type":"idle"}}));
        s.command(Command::FollowUp("t".into(),"Second".into()));assert_eq!(s.outbox.len(),1);
    }
    #[test]fn permission_requests_never_generate_automatic_approval(){
        let mut s=state();s.message(json!({"id":"a","method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"turn","command":"example"}}));assert!(s.outbox.is_empty());
        let key=s.snapshot.threads[0].approvals[0].key.clone();s.command(Command::Deny(key));assert_eq!(s.outbox.pop_front().unwrap()["result"]["decision"],"decline");
    }
    #[test]fn account_read_uses_no_token_refresh_and_blocks_api_key_actions(){
        let mut s=State::new(Snapshot{epoch:8,connection:Connection::Connecting,..Snapshot::unavailable("")});
        s.begin();let initialize=s.outbox.pop_front().unwrap();
        s.message(json!({"id":initialize["id"],"result":{"userAgent":"synthetic"}}));
        assert_eq!(s.outbox.pop_front().unwrap()["method"],"initialized");
        let account=s.outbox.pop_front().unwrap();
        assert_eq!(account["method"],"account/read");assert_eq!(account["params"],json!({"refreshToken":false}));
        assert!(s.outbox.is_empty());
        s.command(Command::CreateThread(r"C:\synthetic".into()));assert!(s.outbox.is_empty());
        s.message(json!({"id":account["id"],"result":{"account":{"type":"apiKey"},"requiresOpenaiAuth":true}}));
        assert!(!s.snapshot.account.is_ready());
        s.command(Command::CreateThread(r"C:\synthetic".into()));s.command(Command::FollowUp("t".into(),"Never".into()));
        assert!(s.outbox.is_empty());
    }
    #[test]fn create_is_explicit_single_flight_read_only_and_ack_registers_exact_id(){
        let mut s=state();s.command(Command::CreateThread(r"C:\synthetic".into()));s.command(Command::CreateThread(r"C:\synthetic".into()));
        assert_eq!(s.outbox.len(),1);assert!(s.snapshot.creating_thread);assert!(s.snapshot.created_thread_id.is_none());
        let create=s.outbox.pop_front().unwrap();assert_eq!(create["method"],"thread/start");
        assert_eq!(create["params"],json!({"cwd":r"C:\synthetic","sandbox":"read-only","approvalPolicy":"untrusted","approvalsReviewer":"user","modelProvider":"openai"}));
        s.message(json!({"id":create["id"],"result":{"thread":{"id":"created","modelProvider":"openai","status":{"type":"idle"}},"sandbox":{"type":"readOnly"},"approvalPolicy":"untrusted","approvalsReviewer":"user","modelProvider":"openai"}}));
        assert!(!s.snapshot.creating_thread);assert_eq!(s.snapshot.created_thread_id.as_deref(),Some("created"));
        assert!(s.loaded.contains("created")&&s.attached.contains("created"));
        assert!(s.snapshot.threads.iter().any(|t|t.id=="created"&&t.subscription_eligible));
        assert!(s.outbox.is_empty());
    }
    #[test]fn create_mismatch_cannot_be_reenabled_by_metadata_refresh_or_reload(){
        let mut s=state();s.command(Command::CreateThread(r"C:\synthetic".into()));let create=s.outbox.pop_front().unwrap();
        let metadata=json!({"id":"created","modelProvider":"openai","status":{"type":"idle"}});
        s.message(json!({"id":create["id"],"result":{"thread":metadata,"sandbox":{"type":"workspaceWrite"},"approvalPolicy":"untrusted","approvalsReviewer":"user","modelProvider":"openai"}}));
        s.metadata("created",&metadata);s.command(Command::FollowUp("created".into(),"Blocked".into()));assert!(s.outbox.is_empty());
        s.message(json!({"method":"thread/closed","params":{"threadId":"created"}}));
        s.loaded.insert("created".into());s.attached.insert("created".into());s.metadata("created",&metadata);
        s.command(Command::FollowUp("created".into(),"Still blocked".into()));assert!(s.outbox.is_empty());
    }
    #[test]fn create_error_releases_guard_and_account_change_invalidates_prompts(){
        let mut s=state();s.command(Command::CreateThread(r"C:\synthetic".into()));let create=s.outbox.pop_front().unwrap();
        s.message(json!({"id":create["id"],"error":{"code":-32600,"message":"synthetic"}}));assert!(!s.snapshot.creating_thread);
        s.command(Command::CreateThread(r"C:\synthetic".into()));assert_eq!(s.outbox.len(),1);
        question(&mut s);let key=s.snapshot.threads[0].questions[0].key.clone();
        s.message(json!({"method":"account/updated","params":{"authMode":"apikey"}}));
        assert_eq!(s.snapshot.account,AccountState::Checking);assert!(!s.snapshot.creating_thread);assert!(s.snapshot.created_thread_id.is_none());
        assert!(s.snapshot.threads.is_empty());assert!(s.prompts.is_empty());assert_ne!(s.snapshot.epoch,key.epoch);
        assert_eq!(s.outbox.len(),1);assert_eq!(s.outbox.front().unwrap()["method"],"account/read");
        s.command(Command::Answer(key,BTreeMap::from([("q".into(),vec!["A".into()])])));assert_eq!(s.outbox.len(),1);
    }
    #[test]fn quota_is_provider_data_and_preview_is_bounded_utf8(){
        assert!(parse_quota(&json!({})).windows.is_empty());assert!(parse_quota(&json!({"rateLimits":{"primary":{"usedPercent":200}}})).windows.is_empty());
        let mut s=state();for _ in 0..20{s.message(json!({"method":"item/agentMessage/delta","params":{"threadId":"t","delta":"🙂".repeat(200)}}));}assert!(s.snapshot.threads[0].last_message.len()<=MAX_PREVIEW);
        s.message(json!({"id":1,"method":"item/tool/call","params":{"threadId":"t","tool":"shell"}}));assert!(s.outbox.is_empty());
    }
    #[test]fn listing_is_not_subscription_and_resume_ack_is_required(){
        let mut s=state();s.attached.remove("t");s.metadata("t",&json!({"id":"t","modelProvider":"openai","status":{"type":"idle"}}));
        s.request("thread/read",json!({"threadId":"t"}),Pending::Read("t".into()));let read=s.outbox.pop_front().unwrap();
        let metadata=json!({"id":"t","modelProvider":"openai","status":{"type":"idle"}});
        s.message(json!({"id":read["id"],"result":{"thread":metadata}}));
        assert!(!s.snapshot.threads[0].subscription_ready);assert!(!s.attached.contains("t"));
        let resume=s.outbox.pop_front().unwrap();assert_eq!(resume["method"],"thread/resume");
        s.command(Command::FollowUp("t".into(),"Too early".into()));assert!(s.outbox.is_empty());
        s.message(json!({"id":resume["id"],"result":{"thread":metadata}}));
        assert!(s.snapshot.threads[0].subscription_ready);assert!(s.attached.contains("t"));
    }
    #[test]fn stale_loaded_reply_cannot_remove_a_just_created_thread(){
        let mut s=state();s.refresh(false);let loaded=s.outbox.pop_front().unwrap();
        s.command(Command::CreateThread(r"C:\synthetic".into()));let create=s.outbox.pop_front().unwrap();
        s.message(json!({"id":create["id"],"result":{"thread":{"id":"new","modelProvider":"openai","status":{"type":"idle"}},"sandbox":{"type":"readOnly"},"approvalPolicy":"untrusted","approvalsReviewer":"user","modelProvider":"openai"}}));
        s.message(json!({"id":loaded["id"],"result":{"data":[]}}));
        assert!(s.snapshot.threads.iter().any(|t|t.id=="new"&&t.subscription_ready));assert!(s.attached.contains("new"));
    }
    #[test]fn history_recovers_missed_completion_but_cannot_overwrite_a_new_turn(){
        let mut s=state();let revision=s.event_versions["t"];
        s.request("thread/turns/list",json!({"threadId":"t"}),Pending::History("t".into(),revision,0));let history=s.outbox.pop_front().unwrap();
        let result=json!({"data":[{"id":"old","status":"completed","items":[{"type":"agentMessage","text":"Synthetic completed marker"}]}]});
        s.message(json!({"id":history["id"],"result":result}));
        assert!(s.snapshot.threads[0].completed);assert_eq!(s.snapshot.threads[0].last_message,"Synthetic completed marker");
        s.request("thread/turns/list",json!({"threadId":"t"}),Pending::History("t".into(),revision,0));let stale=s.outbox.pop_front().unwrap();
        s.message(json!({"method":"turn/started","params":{"threadId":"t","turn":{"id":"new"}}}));
        s.message(json!({"id":stale["id"],"result":result}));
        assert!(!s.snapshot.threads[0].completed);assert_eq!(s.snapshot.threads[0].status,Status::Active);assert!(s.snapshot.threads[0].last_message.is_empty());
    }
    #[test]fn resume_error_diagnostics_never_retain_backend_paths_or_ids(){
        let mut s=state();s.attached.remove("t");s.request("thread/resume",json!({"threadId":"t"}),Pending::Resume("t".into()));let resume=s.outbox.pop_front().unwrap();
        s.message(json!({"id":resume["id"],"error":{"code":-32600,"message":"no rollout found for thread id private-id C:\\private\\secret"}}));
        let diagnostic=s.snapshot.threads[0].subscription_error.as_ref().unwrap();
        assert_eq!(diagnostic.method,"thread/resume");assert_eq!(diagnostic.code,Some(-32600));assert_eq!(diagnostic.reason,"no_rollout");
        assert!(!s.snapshot.threads[0].subscription_ready);
    }
    #[test]fn resume_recovery_clears_only_its_error_and_keeps_followup_failures(){
        let mut s=state();s.attached.remove("t");
        s.request("thread/resume",json!({"threadId":"t"}),Pending::Resume("t".into()));let failed=s.outbox.pop_front().unwrap();
        s.message(json!({"id":failed["id"],"error":{"code":-32600,"message":"no rollout found"}}));
        s.request("thread/resume",json!({"threadId":"t"}),Pending::Resume("t".into()));let recovered=s.outbox.pop_front().unwrap();
        let metadata=json!({"id":"t","modelProvider":"openai","status":{"type":"idle"}});
        s.message(json!({"id":recovered["id"],"result":{"thread":metadata}}));
        assert!(s.snapshot.last_rpc_error.is_none());assert!(s.snapshot.last_error.is_none());
        s.outbox.clear();s.attached.remove("t");
        s.request("thread/resume",json!({"threadId":"t"}),Pending::Resume("t".into()));let failed=s.outbox.pop_front().unwrap();
        s.message(json!({"id":failed["id"],"error":{"code":-32600,"message":"no rollout found"}}));
        s.request("thread/queue/add",json!({}),Pending::Enqueue("t".into()));let followup=s.outbox.pop_front().unwrap();
        s.message(json!({"id":followup["id"],"error":{"code":-32601,"message":"unsupported"}}));let error=s.snapshot.last_error.clone();
        s.request("thread/resume",json!({"threadId":"t"}),Pending::Resume("t".into()));let recovered=s.outbox.pop_front().unwrap();
        s.message(json!({"id":recovered["id"],"result":{"thread":metadata}}));
        assert_eq!(s.snapshot.last_error,error);assert_eq!(s.snapshot.last_rpc_error.as_ref().unwrap().method,"thread/queue/add");
    }
    #[test]fn late_idle_notification_retries_history_with_a_finite_bound(){
        let result=json!({"data":[{"id":"done","status":"completed","items":[{"type":"agentMessage","text":"Recovered"}]}]});
        let mut s=state();s.request_history("t".into(),0);let first=s.outbox.pop_front().unwrap();
        s.message(json!({"method":"thread/status/changed","params":{"threadId":"t","status":{"type":"idle"}}}));
        s.message(json!({"id":first["id"],"result":result}));
        assert!(!s.snapshot.threads[0].completed);let retry=s.outbox.pop_front().unwrap();assert_eq!(retry["method"],"thread/turns/list");
        s.message(json!({"id":retry["id"],"result":result}));assert!(s.snapshot.threads[0].completed);
        let mut bounded=state();bounded.request_history("t".into(),0);
        for _ in 0..3 {
            let read=bounded.outbox.pop_front().unwrap();
            bounded.message(json!({"method":"thread/status/changed","params":{"threadId":"t","status":{"type":"idle"}}}));
            bounded.message(json!({"id":read["id"],"result":result}));
        }
        assert!(bounded.outbox.is_empty());assert!(!bounded.snapshot.threads[0].completed);
    }
}
