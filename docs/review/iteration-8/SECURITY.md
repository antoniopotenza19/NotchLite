# Collegamento Codex — confini e revisione mirata

Data: 2026-11-26. Stato: ricerca e revisione statica completate sulla fonte sotto identificata; i rilievi sono chiusi nel perimetro. Il coordinatore ha completato la verifica finale con due client sul server condiviso e 107 test. Le chat GUI Desktop incorporate restano un caso distinto non verificato dal test standalone.

## Autorizzazione e metodo

L'utente ha autorizzato un collegamento tramite codice/server condiviso senza usare il desktop. Questa ricerca legge soltanto documentazione, fonti pubbliche e codice distribuito nel pacchetto dell'app. Non legge credenziali, cookie, autenticazione, database, conversazioni, log personali, handle di processi o endpoint/token privati. Non modifica l'ambiente di avvio di Desktop, non apre finestre, non avvia server, non invia messaggi e non esegue connessioni al daemon.

La futura revisione del codice è indipendente e in sola lettura; l'unico file posseduto dal revisore in questa iterazione è questo rapporto. Si applicano AGENTS, Ponytail, security-review e il percorso openai-docs.

## Risultati: strumenti host e programma esterno

Gli strumenti offerti all'assistente dalla chat ospitante sono una superficie distinta dal protocollo app-server usabile da NotchLite. La loro presenza non consegna all'eseguibile Rust endpoint o credenziali riutilizzabili. In questa ricerca non sono state invocate letture di chat reali per provare un'integrazione esterna.

La [documentazione ufficiale app-server](https://learn.chatgpt.com/docs/app-server) descrive trasporti e JSON-RPC per costruire client; la [documentazione dei comandi](https://learn.chatgpt.com/docs/developer-commands) descrive CLI --remote e daemon lifecycle. La [documentazione delle connessioni remote](https://learn.chatgpt.com/docs/remote-connections) descrive SSH e handoff: non dimostra un endpoint esterno supportato per le chat già attive sul server incorporato della GUI Windows. Avviare un server separato non migra queste chat né ne trasferisce automaticamente la proprietà.

## Risultati: selettore nel bundle pubblico Windows

È stato letto in memoria soltanto il codice JavaScript distribuito dentro app/resources/app.asar del pacchetto OpenAI.Codex 26.1120.6357.0. Nessuna estrazione o modifica dei dati del profilo utente.

| Codice distribuito | Evidenza statica |
| --- | --- |
| .vite/build/app-server-host.js, offset carattere 579812 | Il ramo che usa CODEX_APP_SERVER_USE_LOCAL_DAEMON richiede esplicitamente una piattaforma diversa da win32, oltre ad altri controlli di host, override e versione. Quel ramo non abilita il collegamento automatico su Windows in questo pacchetto. |
| .vite/build/early-bootstrap-824-73049028835f2e68.js, offset carattere 172932 | Il selettore accetta CODEX_APP_SERVER_WS_URL o websocket_url, salvo forzatura del trasporto CLI. È un override interno presente nel codice: non prova supporto documentato, compatibilità degli strumenti Desktop, o possibilità di connettersi al server incorporato già aperto. Non è stato impostato o eseguito. |

SHA-256 dei due file di codice, letti senza eseguirli:

- app-server-host.js: 1a27128021253ea901a0edab2fc5013920104ca1ad73d7ff7fc4df6e0098cf46.
- early-bootstrap-824-73049028835f2e68.js: c407eae1ccf096a5d63157d33cf6947faf2592bb7160f3505d60350e87b11b04.

Le issue pubbliche [41014](https://github.com/openai/codex/issues/41014) e [47425](https://github.com/openai/codex/issues/47425) segnalano regressioni del collegamento condiviso e problemi degli strumenti dell'app. Sono resoconti di utenti, non istruzioni ufficiali o prove su questa macchina. Nessun workaround di pipe, peer check, symlink o ambiente è stato applicato.

Non è stata identificata una procedura pubblica supportata che colleghi NotchLite alle chat GUI Windows attuali evitando migrazione o creazione di chat sul server condiviso. Questa è una conclusione circoscritta alle fonti cercate e al pacchetto indicato, non l'affermazione che una soluzione futura sia impossibile.

## Vincoli per la revisione dell'implementazione

Il bottone esplicito di avvio server deve descrivere un server condiviso separato; non deve promettere il collegamento delle chat Desktop esistenti. Nessun avvio automatico per il solo fatto di abilitare il pannello.

La revisione controllerà: eseguibile e argomenti strutturati senza shell; worker nascosto con output e attesa limitati; stato busy e arresto del solo sottoprocesso CLI posseduto; JSON lifecycle validato e socket locale canonico confrontato con quello atteso; errori di package incompleto esposti. Il daemon condiviso non deve essere terminato alla chiusura del pannello, perché potrebbe possedere turni attivi.

Il [README pubblico del daemon](https://github.com/openai/codex/blob/main/codex-rs/app-server-daemon/README.md) indica policy proprie per updater e ambiente ereditato. Il solo fatto di non invocare update o bootstrap non prova che il daemon avviato non abbia updater o rete. Non modificare preferenze dell'utente per ottenere implicitamente quel risultato; conservare la distinzione tra azioni di NotchLite e comportamento del servizio avviato.

Per thread/start: azione manuale distinta, cwd esplicita validata, richiesta associata al nuovo thread, nessuna migrazione o resume indiscriminata di chat possedute dalla GUI, nessun bypass delle richieste umane. Concorrenza, risposte tardive e capacità delle code saranno rilette sulla fonte finale.

## Copertura del nuovo codice

| Superficie | Fonte letta e conclusione statica |
| --- | --- |
| Avvio esplicito | codex_server.rs, assist.rs e i chiamanti overlay: StartWorker nasce soltanto dall'azione manuale o dalla modalità CLI dedicata. Abilitare il pannello non avvia il daemon. Il controller rifiuta avvii duplicati, un socket personalizzato o un server già collegato. |
| Launcher | codex_server.rs:38,56: comando strutturato app-server daemon start, stdin nullo, finestra nascosta, stdout massimo 64 KiB. Richiesti status started/alreadyRunning, backend pid, versioni non vuote e socket corrispondente a quello canonico scoperto; il percorso restituito non viene sondato dal parser. |
| Arresto e pipe | Letture fino ai byte disponibili, controllo cancel/deadline, recheck dopo process exit per l'ultimo write. Vengono chiusi soltanto il launcher o il proxy posseduti. Nessun daemon stop/restart alla chiusura del pannello. Il timeout del loop non rende spawn/kill/wait del sistema operativo operazioni con termine assoluto. |
| Account | account/read con refreshToken false prima del catalogo/azioni. Solo tipo chatgpt abilita le azioni, e solo modelProvider openai abilita la chat. Il modello conserva tipo/piano, non email, ID utente o credenziali. Account updated cambia epoch, invalida prompt/pending/outbox e richiede una nuova lettura. È verifica del provider dichiarato dall'endpoint, non un audit del suo billing interno. |
| Nuova chat | create_thread, validate_workspace e Command::CreateThread: cwd assoluta su drive locale, canonicalizzazione e directory esistente; niente UNC/dispositivi o drive remoto. Richiesti read-only, untrusted, reviewer user e provider openai. Thread/start crea il thread senza turno o messaggio automatico. Guardia concorrente e limiti 32; ACK associato alla richiesta pendente e ID restituito. |
| Policy inattese | Il set restricted conserva fino a 32 ID rifiutati per tutta la vita del Bridge, anche attraverso refresh, closed, unload e cambio account. Il limite blocca ulteriori creazioni; una nuova connessione ha stato nuovo. Nessuna quarantena persistente o verifica universale delle policy delle chat preesistenti. Il titolo distingue sola lettura da verifica policy. |
| Diagnostica | codex_diagnostic.rs e main.rs: modalità anticipate prima di COM/mutex/finestre. Connect-check legge solo metadati RPC. Solo self-test esplicito crea un thread e invia due messaggi di prova: il primo materializza il turno prima della sottoscrizione dell'osservatore, il secondo verifica gli eventi condivisi. Stdout contiene esiti e contatori, senza corpi delle chat o credenziali. L'istruzione di non usare strumenti è un prompt, non un blocco tecnico dei tool di lettura disponibili. |
| Controlli UI | Pulsanti e workspace nei percorsi setup/form/command di assist_overlay; gate nel controller e ulteriore controllo nel worker. Il refresh non sovrascrive il workspace in modifica. La selezione del nuovo thread usa l'ID di ACK, senza migrare una chat della GUI. Layout e aritmetica sono stati letti; interazione reale, DPI, focus e accessibilità non sono stati provati. |
| Helper manuale | Setup-CodexSharedServer.ps1: URL ufficiale e versione fissati, SHA-256 prima di tar/eseguibili, limiti/controlli dei nomi e rifiuto link archivio, directory di estrazione GUID nuova, verifica manifest/helper/versione. Nessun avvio senza -Start; niente login, autorun o PATH globale. La verifica riguarda il pacchetto fissato, non archivi arbitrari scelti dall'utente. |
| Sottoscrizione confermata | protocol.rs:168,174 e codex_bridge.rs:153: catalogo/read e richiesta resume pendente non costituiscono ready. Ready deriva da ACK Create o Resume; nessun resume duplicato pendente per ID. UI, follow-up pubblico/worker e diagnostica richiedono ready. Un errore di resume mantiene il singolo thread non pronto. |
| Cronologia e topologia | protocol.rs:151,187,225,322: il risultato Loaded porta la revisione di topologia, quindi una risposta precedente alla creazione/chiusura non sostituisce il catalogo aggiornato. History porta la versione degli eventi del thread; non sovrascrive un turno nuovo. Il completamento viene recuperato solo quando Idle senza prompt/approval, in base allo status effettivo dell'ultimo turno. Eventuali retry per il conflitto con Idle riguardano solo la lettura history e sono limitati a due, senza inviare di nuovo messaggi. |
| Errori minimizzati | protocol.rs:375: RpcError conserva solo metodo statico, codice numerico e reason da whitelist. Il messaggio backend viene letto per classificare e poi scartato, senza conservare percorsi, ID o corpi. Il successo di resume ripulisce il suo errore solo quando non restano errori di sottoscrizione; un errore queue/add successivo non viene nascosto. |

Restano le protezioni del bridge già riesaminate nell'iterazione 7: chiavi domanda epoch/request/thread/turn, rifiuto delle risposte obsolete, nessuna concessione automatica di permessi, follow-up solo queue/add e queue/start con guardia per chat, dimensioni di messaggi/code limitate e terminazione del solo proxy posseduto. Il nuovo codice non aggiunge acquisizione audio o provider Audio AI.

## Registro dei rilievi

| ID | Stato | Evidenza e tentativo di confutazione |
| --- | --- | --- |
| A8-1 | Confermato, corretto e riletto | CreateThread rilevava sandbox/approval/reviewer inattesi e disabilitava subscription_eligible, ma metadata lo riassegnava usando solo modelProvider. Il controllo dell'account non confutava il caso. Ora restricted per ID è consultato anche da metadata e sopravvive a chiusura/ricaricamento; il test simula mismatch, refresh, closed/reload e follow-up rifiutato. Nessuna riproduzione attiva di attacco. |
| A8-2 | Incompletezza della prima implementazione, chiusa | valid_start_reply accettava qualunque oggetto JSON e check_local constatava solo il percorso presente. La fonte riletta valida i campi lifecycle e confronta il socket canonico atteso, prima di dichiarare il launcher riuscito. Il test rifiuta anche oggetto vuoto, JSON multipli, percorso diverso e risposta troppo grande. Non era un exploit dimostrato o un difetto di un rilascio già congelato. |
| A8-3 | Correttezza helper, corretta e riletta | IsPathRooted ammetteva C:relative e percorsi radicati dipendenti dal drive corrente; GetFullPath li risolveva prima della scrittura, non li rigettava. La documentazione Microsoft conferma la distinzione rooted/fully qualified. Ora la regex richiede drive assoluto prima di New-Item e DriveInfo rifiuta Network. Non implica una prova di escape dall'archivio, che è fissato da hash. |
| A8-4 | Difetto funzionale osservato dal coordinatore, corretto e riletto; verifica finale riuscita | Il tentativo intermedio mostrava C1 completato e C2 Idle con marker ma senza completed: History recuperava l'anteprima, senza ripristinare il flag di completamento. Inoltre la presenza nel catalogo precedeva la conferma di sottoscrizione. Il vecchio PASS non confutava quel fallimento. La fonte finale separa ready dall'elenco, recupera lo status dall'ultimo turno e protegge cronologia/catalogo da eventi e risposte obsolete. Sei nuovi test sintetici verificano ACK, topologia, recupero, mancata sovrascrittura di nuovo turno, whitelist errori, pulizia corretta e retry limitato. Il nuovo self-test con due messaggi conferma il recupero iniziale e gli eventi condivisi. Non è presentato come una vulnerabilità di sicurezza o come prova delle chat GUI esistenti. |

Per A8-3: [Microsoft, Path.IsPathRooted](https://learn.microsoft.com/en-us/dotnet/api/system.io.path.ispathrooted). Il controllo del drive non attesta la destinazione di ogni possibile reparse point modificato in modo concorrente; lo script manuale assume una directory scelta e controllata dall'utente.

Anche un avvio cancellato può avere già creato il daemon; il messaggio lo dichiara. L'assenza di un comando di stop di NotchLite non garantisce che servizio, updater o sistema operativo lo mantengano attivo indefinitamente.

## Evidenze e limiti

Il revisore non ha avviato build, daemon, GUI, socket, acquisizioni, messaggi o scanner. Il coordinatore ha eseguito i test e le prove funzionali. I log finali letti riportano 107 test passati, zero falliti in 0,51 s e release GNU riuscita in 29,74 s. Non equivalgono a una prova Windows MSVC o GUI.

L'ultimo snapshot include soltanto due righe di fixture privacy sotto cfg(test) in assist.rs: gli esempi di deep link usano un UUID sintetico. La rilettura di questa delta non ha trovato modifiche al codice di produzione. Il rebuild ha comunque cambiato l'hash dell'eseguibile: il coordinatore ha quindi ripetuto controllo connessione e self-test reale sull'esatto binario finale SHA-256 139df501535225d6e193924272a697902d1e47b288520e3458d3d010a0511a53. L'ultima prova conferma account ChatGPT/quota, recupero history, due client sulla stessa chat, risposta, completamento e presenza dei contatori token, con due messaggi e nessuna GUI. Non occorrono ulteriori model run per questo snapshot; nessuna prova live è stata eseguita dal revisore.

Le summary del coordinatore registrano: avvio server riuscito, controllo connessione ChatGPT e quota riuscito, helper verificato da archivio in cache senza -Start. Il coordinatore riporta anche tre casi negativi di validazione percorso passati e verifica positiva del helper in 4,8 s. Il self-test finale crea una chat di prova e invia due messaggi: verifica historyRecoveryVerified, due client sulla stessa chat, marker, completamento e presenza dei contatori token in entrambi. guiOpened è false. Sono state lette soltanto le summary generate; qui non si pubblicano ID, corpi, dati account o conteggi specifici. I tentativi precedenti, incluso il timeout intermedio, sono contesto della correzione A8-4; la conclusione usa il nuovo PASS finale, non il vecchio risultato.

Il primo avvio ufficiale e il secondo alreadyRunning sono riferiti dal coordinatore; il reviewer non ha ispezionato autenticazione o dati personali. Avvio con successo e visibilità delle nuove chat standalone non dimostrano accesso alle chat GUI Desktop attuali. La ricerca non ha trovato una via pubblica supportata per quelle chat senza cambiamento di topologia.

Restano non provati: GUI, DPI, tastiera/focus, multi-monitor reale, continuità degli strumenti interni Desktop, consumo RAM/CPU e latenza, tutte le combinazioni di versioni e policy dei daemon futuri. Nessun certificato generale di sicurezza del prodotto.

## SHA-256 delle fonti rilette

| File | SHA-256 |
| --- | --- |
| src/codex_server.rs | 8417d1cc4112a511c6a6f26f566f56517a040e262c113f3aecb35fa06e9f5168 |
| src/codex_diagnostic.rs | 4e6a418ca4ce1f804ddab8ff357ccfe8db066e42419c8a39bbf5abc4e8e8c99a |
| src/codex_bridge.rs | c90879795d14a51fbec2ccca220866dc33f5fedaa2a432d1c10bd377a90cf8c5 |
| src/codex_bridge/protocol.rs | 28477d944353dff7610ea5ef256d71e949efaea1f9ae479c5755c61d90f43f01 |
| src/codex_bridge/transport.rs | 595a54a9d48841a1a397597f840defff67cd8c726363d79162238e32569f6d10 |
| src/assist.rs | 9fc0201456bd589296f70c02d0253f63f07c09c425caa02b966ce41c99b34f9f |
| src/assist_overlay.rs | f41a43b0ccd13875e258d1f8371eeeba5cf8b0af3138568aea48dedd894c4328 |
| src/assist_layout.rs | a8415ee6c0e70b76aaaf4c73386c60355a61c0c2dd4ecc66a43a664b77f46773 |
| src/main.rs | 3c5979553f36da2b8844fef0a24e8e5ee60c6ddbb801cb3237253def9913d7de |
| tools/Setup-CodexSharedServer.ps1 | 948cc895e5c2d6cc430cd1a9947f318305db4f36fff64c68d78a15a0e17c31eb |

Verdetto: nessun blocco aperto individuato nel perimetro; i rilievi sono corretti e la verifica funzionale finale del coordinatore sul server condiviso è riuscita. Il caso delle chat Desktop incorporate rimane distinto e non supportato dalla prova standalone. Questo rapporto non certifica l'intero prodotto.
