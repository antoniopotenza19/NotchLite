# Dashboard delle chat Codex

Il bridge in `src/codex_bridge/` collega il pannello al **daemon Codex locale
già in esecuzione**. Il pulsante separato **Avvia server locale** richiama il
gestore ufficiale soltanto su click; **Nuova chat** crea una sessione senza
inviare messaggi. Il programma non legge database, rollout privati, cookie o file di autenticazione. Le chat di
ChatGPT non fanno parte di questa integrazione.

## Collegamento e disponibilità

È necessario il CLI ufficiale `codex.exe`: vengono cercati prima i pacchetti
ufficiali gestiti in `CODEX_HOME/packages/app-server-daemon/current/bin` e
`packages/standalone/current/bin`, poi nel PATH o come App Execution Alias
in `%LOCALAPPDATA%\Microsoft\WindowsApps`, e un daemon che esponga il
control socket ufficiale. Il CLI esaminato è `0.171.0-alpha.13.2`, incluso
nell'app Codex Windows `26.1120.6357.0`.
L'integrazione può anche passare un percorso assoluto esplicito a
`Bridge::start_with_cli`; non esegue PowerShell per cercarlo.
L'impostazione facoltativa `codex_cli` consente lo stesso percorso esplicito.
`check_local` verifica soltanto metadati di percorso e presenza del CLI,
senza connessioni, processi o lettura di chat. È usato dal controllo headless.

La scoperta usa `CODEX_HOME`, oppure `%USERPROFILE%\.codex`, e il percorso
ufficiale `app-server-control/app-server-control.sock`. Il valore facoltativo
`codex_socket` può indicare un altro percorso assoluto locale. Il parent viene
canonicalizzato; percorsi UNC e indirizzi oltre il limite AF_UNIX Windows di
107 byte vengono rifiutati. Vengono letti soltanto i metadati del percorso.

Il bridge avvia, senza finestra, esclusivamente questo sottoprocesso:

```text
codex.exe app-server proxy --sock <percorso-socket>
```

`proxy` inoltra byte al control socket: il client esegue il handshake WebSocket
e invia JSON-RPC nei frame del tunnel. Questo è diverso dal JSONL di un nuovo
`codex app-server --listen stdio://`. Non viene aperto un listener TCP/LAN.
La protezione del socket è quella del sistema operativo e dell'installazione
Codex; non viene modificata dal pannello.

Se Codex Desktop usa un server incorporato senza socket condiviso, il pannello
mostra **non disponibile**. La presenza di chat salvate nello stesso account
non dimostra che un altro processo possa ricevere i loro eventi attivi.
`daemon start` richiede il click su **Avvia server locale** e un pacchetto
ufficiale completo. Non vengono eseguiti `bootstrap`, `restart` o login
automatici. Una connessione interrotta richiede
un nuovo collegamento esplicito; non vengono reinviati messaggi automaticamente.

Il 26 novembre è stato preparato il pacchetto ufficiale completo ed è stato
avviato un daemon locale: il bridge si collega realmente, verifica l'accesso
ChatGPT e legge le quote. L'app Desktop già aperta usa il suo server
incorporato: **le sue chat GUI attive restano separate da questo bridge**.
Non è stata individuata un'impostazione o una procedura pubblica
supportata per spostare la GUI Windows già avviata sullo stesso daemon locale.
Il comando `/daemon` documentato appartiene al CLI, non al composer Desktop.
Avviare un nuovo daemon o riaprire semplicemente l'app non dimostra che quelle
chat diventino condivise. Il pannello non mostra una dashboard fittizia.

Per la preparazione ripetibile e i comandi headless vedere
[CODEX-SHARED-SERVER.md](CODEX-SHARED-SERVER.md). Il launcher ha timeout di
20 secondi, output limitato a 64 KiB e verifica il risultato JSON e il socket
canonico. La cancellazione termina soltanto il launcher posseduto: se il
daemon è già partito, rimane attivo. La chiusura del pannello non ferma il
daemon né gli altri client. Un avvio scaduto richiede verifica, senza riavvii.

### Strumenti collegati alla chat di sviluppo

Il 26 novembre 2026 sono stati verificati, senza usare il desktop, gli
strumenti Codex messi a disposizione dell'assistente in questa chat:
`list_threads` e `get_usage_limits` leggono rispettivamente l'elenco delle
chat e i limiti dell'account corrente. Queste letture funzionano anche quando
il bridge esterno non trova il socket condiviso. I dati privati restituiti
non sono inclusi nel repository o nel pacchetto.

Questo accesso appartiene all'ambiente che ospita l'assistente: non fornisce
al programma Rust un endpoint o credenziali riutilizzabili. Non è stato
individuato un collegamento pubblico supportato con cui NotchLite possa
invocare autonomamente quegli strumenti. La verifica delle letture qui non
equivale quindi a una connessione del pannello alle chat Desktop. Non sono
stati inviati messaggi o risposte alle chat reali durante questa verifica.

Il protocollo è utilizzabile quando il thread risiede già su un daemon locale
condiviso supportato. Il CLI documenta `--remote unix://<percorso>` per
collegarsi esplicitamente a quel server; questo descrive un client CLI e non
una migrazione delle chat Desktop esistenti. Non viene eseguito da NotchLite.

Per aprire una chat nell'app originale è documentato
`codex://threads/<thread-id>` (ID tecnico del thread). È un collegamento di
navigazione: non sottoscrive il bridge né invia messaggi. Un ID sconosciuto
richiede apertura generale dell'app, senza costruire un percorso arbitrario.

## Dati mostrati

Dopo `initialize` viene eseguito `account/read` con `refreshToken: false`.
Le azioni richiedono account di tipo ChatGPT e provider OpenAI: le API key e
gli altri provider non costituiscono un fallback. I dati identificativi
dell'account non vengono copiati nello stato del pannello. Un cambio account
invalida richieste, risposte pendenti e catalogo della connessione.

**Nuova chat** richiede una cartella locale esistente con percorso assoluto.
`thread/start` imposta sandbox `read-only`, policy `untrusted`, revisore
`user` e provider `openai`. L'ACK deve confermare queste policy, altrimenti
l'invio resta bloccato per quella chat nella connessione corrente. Il click
crea solo la chat: il primo messaggio richiede **Send**. Sola lettura non
significa che Codex sia privo di strumenti: i turni successivi seguono le
policy del client ufficiale; NotchLite non concede autorizzazioni automatiche.

Il protocollo esegue `initialize`/`initialized`, quindi legge gli ID già
caricati con `thread/loaded/list` e i metadati con `thread/read`.
Per gli ID restituiti come caricati usa `thread/resume` con il solo ID e
`excludeTurns: true`: questa operazione sottoscrive eventi e può riprodurre
domande ancora pendenti sullo stesso server. Non cambia sandbox, modello,
istruzioni o politiche dei permessi. Legge poi soltanto l'ultimo turno per
ottenere un'anteprima iniziale. Il controllo "già caricato" è una fotografia:
il protocollo non offre un'opzione atomica `resumeOnlyIfLoaded`; tra elenco e
resume una chat inattiva può essere scaricata dal daemon.

Stati, completamenti e testo arrivano da `thread/status/changed`, `turn/*` e
`item/agentMessage/*`. I token sono i contatori riportati da
`thread/tokenUsage/updated`. La quota è esclusivamente il valore del provider
in `account/rateLimits/read` e relative notifiche: un campo assente rimane
non disponibile. Quota dell'account, token del thread e RAM/CPU del processo
sono misure diverse; questo bridge non inventa consumi RAM per singola chat.
Un turno completato indica lo stato del protocollo, non una verifica autonoma
del risultato del lavoro dell'agente.

Le chat caricate vengono aggiornate ogni 5 secondi, la quota ogni 15 secondi.
Limiti del pannello: 32 chat, anteprima di testo 2 KiB, 32 richieste pendenti,
3 domande per richiesta, 32 opzioni per domanda. Testo delle opzioni non
rappresentabile senza alterarne il valore richiede risposta nell'app Codex.
Eventi oltre 512 KiB chiudono il collegamento con un errore esplicito. Non viene
conservata una copia della cronologia sul disco di NotchLite.

## Risposte e follow-up

Una domanda umana è un `item/tool/requestUserInput` con ID RPC proprio.
La risposta usa quell'ID e la mappa degli ID delle domande, senza creare un
nuovo turno. Ogni azione porta `AnswerKey`: epoca della connessione, ID RPC,
thread e turno. La chiave viene verificata al momento del click e nuovamente
nel worker; `serverRequest/resolved` o il completamento del turno la rimuovono.
Una risposta a una vecchia connessione non viene riutilizzata dopo il
ricollegamento, anche se il server riusa un ID numerico.

Il bridge non esegue richieste di tool e non approva permessi. Per richieste
di esecuzione o modifica file può inviare soltanto **Rifiuta**, su click
esplicito; le altre operazioni richiedono apertura di Codex. Non vengono
concesse approvazioni di sessione o regole persistenti. Una domanda segreta
resta marcata `is_secret` per consentire al pannello di oscurarne l'input.

Un follow-up richiede un click e una chat visualizzata come inattiva. Usa
`thread/queue/add` con identificativo univoco del messaggio, seguito da
`thread/queue/start`, che sul server usa `StartIfIdleSubmission`.
Se un altro client ha avviato un turno nel frattempo, il messaggio rimane in
coda sul PC. Non viene usato `turn/start` come fallback: quel metodo può fare
steering di un turno attivo. Un errore dopo l'invio non produce un retry,
per evitare duplicati; il pannello invita a verificare la coda in Codex.
Fino alla conferma o all'errore del primo invio è consentito un solo follow-up
in corso per chat: un secondo click mostra **Invio già in corso**. La guardia
vale anche tra accodamento e avvio; non confronta il testo dei messaggi, quindi
un successivo invio esplicito con lo stesso testo rimane possibile.
I metodi della coda sono sperimentali: su server incompatibili l'invio mostra
un errore e richiede continuazione nell'app originale.

## Arresto e limiti di verifica

Il bridge è creato su richiesta del pannello. Una coda comandi sincrona
contiene al massimo 16 azioni. Un worker gestisce protocollo e scadenze;
un piccolo writer con coda di quattro blocchi da 4096 byte evita che pipe
in stallo impediscano di leggere o verificare le scadenze. Il watchdog del
worker interrompe scritture ferme per 5 secondi; handshake 5 secondi e assenza
di qualsiasi risposta per 45 secondi terminano il collegamento. Chiusura e
Drop terminano soltanto il proxy posseduto e attendono l'uscita dei worker.
Il daemon e le chat Codex rimangono in esecuzione.

I test del modello usano soltanto messaggi JSON sintetici: chiavi scadute,
richieste già risolte, corrispondenza delle risposte, accodamento senza
steering, rifiuto senza autoapprovazione e limiti di testo/quota.
I test unitari del bridge non avviano il CLI né si connettono a chat reali.
Separatamente sono state eseguite prove headless del collegamento reale;
esiti e limiti sono nel [record corrente](review/iteration-8/VALIDATION.json).
Non sono stati eseguiti desktop, GUI o acquisizione audio.

## Audio e abbonamento OpenAI

Lo schema sperimentale del CLI include `thread/realtime/start`,
`appendAudio`, `appendText` e `stop`; include anche `outputModality: text`.
Questo non dimostra che il solo abbonamento ChatGPT abiliti il percorso.
Nella fonte ufficiale esaminata il trasporto realtime WebSocket richiede una
API key e restituisce un errore quando non la trova; il commento di upstream
segnala esplicitamente il limite corrente per sessioni ChatGPT/SIWC.
Il ramo WebRTC non costituisce evidenza di entitlement sull'account dell'utente.

Inoltre il realtime Codex può inoltrare handoff al normale agente della chat.
`includeStartupContext: false`, un prompt e output testuale non costituiscono
un blocco tecnico degli strumenti. Questo bridge non implementa realtime,
non legge `OPENAI_API_KEY`, non cattura audio e non avvia un provider API
con addebito separato come fallback all'abbonamento richiesto.

## Fonti e riproducibilità

Sono stati eseguiti soltanto `codex --version`, `--help`, gli help di
`app-server`, `proxy`, `daemon` e `queue`, e la generazione locale degli
schemi stabili/sperimentali. Le firme sono state confrontate con gli schemi
generati dal CLI installato. Le fonti seguenti sono upstream ufficiali;
il codice corrente di upstream può differire dal binario alpha installato.

- [Documentazione app-server](https://learn.chatgpt.com/docs/app-server).
- [Deep link delle chat Desktop](https://learn.chatgpt.com/docs/app/commands#deeplinks).
- [Comandi sviluppatore, distinti per Desktop e CLI](https://learn.chatgpt.com/docs/developer-commands).
- [Fonte Codex fissata per la verifica](https://github.com/openai/codex/tree/af71942309704997f4bdc86c020717646423c080).
- [Percorso ufficiale control socket](https://github.com/openai/codex/blob/af71942309704997f4bdc86c020717646423c080/codex-rs/app-server-transport/src/transport/mod.rs#L56).
- [Proxy raw stdio](https://github.com/openai/codex/blob/af71942309704997f4bdc86c020717646423c080/codex-rs/cli/src/app_server_proxy.rs).
- [Daemon Windows e fallback al server incorporato](https://github.com/openai/codex/blob/af71942309704997f4bdc86c020717646423c080/codex-rs/app-server-daemon/README.md).
- [Replay delle richieste pendenti](https://github.com/openai/codex/blob/af71942309704997f4bdc86c020717646423c080/codex-rs/app-server/src/request_processors/thread_lifecycle.rs#L1360).
- [Coda e avvio solo se inattivo](https://github.com/openai/codex/blob/af71942309704997f4bdc86c020717646423c080/codex-rs/app-server/src/request_processors/thread_queue_processor.rs#L200).
- [Autenticazione del realtime WebSocket](https://github.com/openai/codex/blob/af71942309704997f4bdc86c020717646423c080/codex-rs/core/src/realtime_conversation.rs#L2488).
- [Uso del piano per app open source: non dà accesso alle conversazioni ChatGPT](https://developers.openai.com/siwc/token-sharing-open-source).

La disponibilità degli strumenti interni di lettura chat nell'ambiente Codex
di sviluppo non è una API da redistribuire con NotchLite. Il prodotto usa
esclusivamente il protocollo app-server pubblico descritto sopra.
