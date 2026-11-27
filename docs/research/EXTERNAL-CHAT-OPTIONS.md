# Chat esterne Codex e Claude: opzioni verificate

Ricerca del 27 novembre 2026 su repository originali, README, licenze e
implementazioni fissate a commit. Nessuna installazione, accesso ai token
utente, chiamata AI, apertura del desktop o importazione di codice.

Per NotchLite la strada più contenuta è conservare il bridge Codex ufficiale
già presente e aggiungere in seguito un adattatore Claude separato. Per i
contatori storici di entrambi, **ccusage** è il candidato più vicino al nostro
obiettivo. Per una chat esterna completa già pronta, **CloudCLI** è il
riferimento funzionale più diretto; **Happy** aggiunge il passaggio fra PC e
telefono. Non sono misure comparative della loro RAM.

| Progetto | Chat interattive | Consumi verificati nel sorgente | Windows e costo strutturale | Licenza del pin |
| --- | --- | --- | --- | --- |
| [CloudCLI / Claude Code UI](https://github.com/siteboon/claudecodeui) | Claude Code e Codex, streaming e ripresa di sessione | Token e finestra di contesto di entrambi; non ho trovato un contatore universale della quota abbonamento | Server Node/React, desktop Electron opzionale; README offre Windows | AGPL-3.0-or-later, termini aggiuntivi nel LICENSE |
| [Happy](https://github.com/slopus/happy) | Wrapper CLI Claude/Codex con client e relay; sessioni continuabili da altri dispositivi | Eventi token Codex; finestre limite Claude mediante API sperimentale e rate-limit events | CLI Node, relay e client separati; desktop Windows in altro repository | MIT |
| [AionUi](https://github.com/iOfficeAI/AionUi) | UI unificata per Claude Code e Codex; backend separato AionCore | UI per token, contesto e costo riportato dall'agente; quota uniforme non verificata | Frontend Electron; backend Rust separato, supporto Windows dichiarato | Apache-2.0 nel frontend; discrepanza da chiarire nel backend |
| [ccusage](https://github.com/ccusage/ccusage) | Nessun invio messaggi: è un lettore di utilizzo | Token di Claude e Codex dai log, report per sessione/giorno e costi API equivalenti | Motore Rust, pacchetti nativi Windows x64/ARM64 e output JSON | MIT |

“Consumo” richiede quattro etichette distinte: **token della sessione**,
**costo stimato**, **quota dell'account** e **CPU/RAM del processo**. Nessuno
dei risultati letti dimostra da solo un pannello affidabile che li copra tutti
per entrambi i fornitori. I valori mancanti devono restare non disponibili,
senza trasformare token o percentuale di contesto in quota residua.

## CloudCLI: riferimento per una chat completa

Pin `de4c768b04a162aa81667086e5599cab5713699a`, versione `1.37.11`.
Il runtime Codex importa `@openai/codex-sdk`, riprende thread con
`resumeThread` e trasmette gli eventi alla UI. Il runtime Claude importa
`@anthropic-ai/claude-agent-sdk` e gestisce lo stream del provider.
Il servizio dei token legge snapshot Codex `token_count` e usage delle
risposte Claude, mentre la UI mostra i token e il dettaglio di contesto.
Questo dimostra un'implementazione oltre alla promessa del README.
[Runtime Codex](https://github.com/siteboon/claudecodeui/blob/de4c768b04a162aa81667086e5599cab5713699a/server/modules/providers/list/codex/codex-runtime.provider.ts),
[runtime Claude](https://github.com/siteboon/claudecodeui/blob/de4c768b04a162aa81667086e5599cab5713699a/server/modules/providers/list/claude/claude-runtime.provider.ts),
[servizio token](https://github.com/siteboon/claudecodeui/blob/de4c768b04a162aa81667086e5599cab5713699a/server/modules/providers/services/provider-token-usage.service.ts).

Il README consente uso locale o remoto e distingue il server open source
dall'offerta cloud opzionale. Il pacchetto include anche Electron per desktop.
Non serve un ambiente ospitato per la variante locale, ma richiede comunque
un server Node aggiuntivo. Non è stata misurata la RAM e non lo descriviamo
come più leggero di NotchLite. Licenza attuale copyleft: prima di riutilizzare
sorgenti nel nostro progetto Apache occorre valutare compatibilità e termini;
non è stata fatta alcuna copia.
[README](https://github.com/siteboon/claudecodeui/blob/de4c768b04a162aa81667086e5599cab5713699a/README.md),
[package](https://github.com/siteboon/claudecodeui/blob/de4c768b04a162aa81667086e5599cab5713699a/package.json),
[licenza](https://github.com/siteboon/claudecodeui/blob/de4c768b04a162aa81667086e5599cab5713699a/LICENSE).

## Happy: continuità PC/telefono, con un relay

Pin `df180efe57d1de25fb0922167dc25cb8ead1868d`, CLI `1.2.7`.
Il client Codex avvia `codex app-server --listen stdio://`, gestisce
`thread/resume`, `turn/start`, richieste di approvazione e notifiche
`thread/tokenUsage/updated`. La CLI originale e i client non coincidono
con l'app desktop più recente, che ha un repository separato. Il relay è
un componente esplicito; il README dichiara cifratura fra dispositivi,
che questa lettura non certifica con un audit crittografico.
[Client Codex](https://github.com/slopus/happy/blob/df180efe57d1de25fb0922167dc25cb8ead1868d/packages/happy-cli/src/codex/codexAppServerClient.ts),
[architettura](https://github.com/slopus/happy/blob/df180efe57d1de25fb0922167dc25cb8ead1868d/README.md),
[CLI e server configurabile](https://github.com/slopus/happy/blob/df180efe57d1de25fb0922167dc25cb8ead1868d/packages/happy-cli/README.md).

Per Claude la quota iniziale è richiesta attraverso un metodo denominato
`usage_EXPERIMENTAL_MAY_CHANGE_DO_NOT_RELY_ON_THIS_API_YET`; eventi successivi
aggiornano le finestre. È una dipendenza sperimentale, non una garanzia che
la quota di Claude resti leggibile in ogni versione o piano. La sorgente
mobile contiene anche un modello per finestre account, ma non basta per
dimostrare uguale copertura end-to-end per tutti i provider.
[Acquisizione limiti Claude](https://github.com/slopus/happy/blob/df180efe57d1de25fb0922167dc25cb8ead1868d/packages/happy-cli/src/claude/claudeRemote.ts),
[normalizzazione](https://github.com/slopus/happy/blob/df180efe57d1de25fb0922167dc25cb8ead1868d/packages/happy-cli/src/claude/utils/usageLimits.ts),
[MIT](https://github.com/slopus/happy/blob/df180efe57d1de25fb0922167dc25cb8ead1868d/LICENSE).

## AionUi: alternativa completa, più ampia del nostro pannello

Frontend pin `e108b6b56b1a6eb99a00904878d0582f657b2908`, versione `2.2.2`.
Il frontend Electron presenta agenti esterni, WebUI e un indicatore che
mostra token, contesto e costo di sessione quando ricevuti. Il backend
AionCore al pin `503a87199eb978790ca77675b39c84b241535bf3` contiene connessioni
reali per Codex JSON-RPC e Claude CLI, con avvio/ripresa e permessi.
Non è stata provata l'esecuzione dell'accoppiata su Windows.
[Indicatore utilizzo](https://github.com/iOfficeAI/AionUi/blob/e108b6b56b1a6eb99a00904878d0582f657b2908/packages/desktop/src/renderer/components/agent/ContextUsageIndicator.tsx),
[backend Codex](https://github.com/iOfficeAI/AionCore/blob/503a87199eb978790ca77675b39c84b241535bf3/crates/aionui-session/src/backend/codex_conn.rs),
[backend Claude](https://github.com/iOfficeAI/AionCore/blob/503a87199eb978790ca77675b39c84b241535bf3/crates/aionui-session/src/backend/claude_conn.rs).

È una workspace applicativa con molti moduli, non un piccolo adattatore
già isolato. Nel backend il LICENSE riporta Apache-2.0 mentre il campo
workspace di Cargo dichiara MIT: prima di copiare parti serve chiarire
questa discrepanza. Nessun riuso effettuato.
[Frontend e piattaforme](https://github.com/iOfficeAI/AionUi/blob/e108b6b56b1a6eb99a00904878d0582f657b2908/readme.md),
[LICENSE backend](https://github.com/iOfficeAI/AionCore/blob/503a87199eb978790ca77675b39c84b241535bf3/LICENSE),
[Cargo backend](https://github.com/iOfficeAI/AionCore/blob/503a87199eb978790ca77675b39c84b241535bf3/Cargo.toml).

## ccusage: candidato per i contatori locali

Pin `479c75abe05678b7168a5da5d971d7540fc7da5b`, versione `20.1.2`.
Il report unificato legge entrambi gli agenti; JSON permette di alimentare
un pannello nativo. Il parser Codex distingue contatori cumulativi, delta,
compaction e copie di cronologia per evitare conteggi doppi quando i dati
lo consentono. Dati mancanti o log cancellati possono far divergere il
totale locale da quello dell'account.
[README](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/README.md),
[parser Codex](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/rust/adapters/codex/src/parser/usage.rs),
[regole dati](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/rust/adapters/codex/src/README.md).

Codex non registra un prezzo USD: il costo calcolato usa token e listini,
non la fattura o il consumo monetario dell'abbonamento. `--mode display`
restituisce zero costo per Codex mantenendo i token; prezzi mancanti e
stime devono restare distinguibili. Il motore è Rust e il package elenca
binari Windows nativi, ma non sono stati scaricati o eseguiti.
[Costi](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/docs/guide/cost-modes.md),
[pacchetti nativi](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/apps/ccusage/package.json),
[MIT](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/LICENSE).

Le release correnti dichiarano una telemetria giornaliera di adozione,
senza contenuto dei log; `--offline` oppure `DO_NOT_TRACK=1` la disabilitano.
Per un adattatore NotchLite proporrei esecuzione locale esplicita e offline,
cache dei soli aggregati, limiti di tempo/output e nessuna installazione
automatica da `latest`.
[Telemetria e offline](https://github.com/ccusage/ccusage/blob/479c75abe05678b7168a5da5d971d7540fc7da5b/README.md).

## Autenticazione, chat già aperte e integrazione proposta

Il protocollo ufficiale Codex espone ripresa delle sessioni, eventi token
e limiti dell'account. Il bridge NotchLite usa già questi canali: è il punto
da estendere per una vista comune, senza includere un secondo frontend.
La disponibilità varia con versione e autenticazione; per dati non
restituiti usare “non disponibile”.
[Codex App Server](https://learn.chatgpt.com/docs/app-server).

Per Claude, l'SDK ufficiale offre sessioni, streaming e richieste utente.
La documentazione attuale precisa però che, salvo approvazione Anthropic,
uno sviluppatore terzo non può offrire login claude.ai o rate limit del
piano nel proprio prodotto. I README di alcune UI dichiarano riuso degli
abbonamenti: non abbiamo verificato le loro eventuali approvazioni. Non
trasformiamo quelle dichiarazioni in una promessa per NotchLite e non
estraiamo credenziali dalle applicazioni.
[Claude Agent SDK, autenticazione](https://code.claude.com/docs/en/agent-sdk/overview).

Nessun repository qui è stato verificato come controllo universale delle
chat già attive nelle app desktop Claude/Codex. Leggere file di cronologia
o riprendere un ID tramite un nuovo processo non dimostra condivisione live
della sessione già caricata altrove. Occorrono prove dedicate del trasporto
e della proprietà della sessione prima di abilitare risposte concorrenti.

Proposta per la prossima integrazione: modello UI comune per sessioni,
messaggi, domande, stato e token; adattatore Codex sul bridge esistente;
adattatore Claude opt-in con autenticazione supportata e verifica della
sessione; ccusage opzionale per aggregati storici. CPU/RAM restano contatori
OS dei processi, distinti dai token. In questa iterazione è stata fatta solo
la ricerca: nessuna dipendenza o integrazione chat Claude è stata aggiunta.
