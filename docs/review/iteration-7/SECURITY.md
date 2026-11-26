# Revisione mirata — dashboard Codex e preparazione chiamate

Data: 2026-11-26. Ambito: bridge locale, controlli umani, risorse per processo e catalogo audio di soli metadati. Metodo: lettura statica di sorgenti, chiamanti, test e confini del protocollo, secondo AGENTS, Ponytail e security-review locali. Il revisore è indipendente per bridge/controller/metriche; è autore del modulo call_audio, per il quale questo documento registra controlli di implementazione, non una seconda revisione indipendente.

La modifica collega un pannello facoltativo a un daemon Codex esistente attraverso il proxy CLI ufficiale. Il pannello chiamate mostra candidate ricavate dai metadati Windows. La scelta esplicita dell'utente per questa versione è solo abbonamento: Audio AI disattivato, senza ascolto, campioni PCM, microfono, trascrizione o invio di audio.

## Esito

Nessun difetto bloccante aperto individuato nel perimetro letto. Un limite mancante sul lavoro del drain degli eventi audio è stato corretto durante questa revisione. Il coordinatore ha inoltre corretto il doppio invio concorrente per chat; la correzione finale è stata riletta indipendentemente. La validazione CLI finale del coordinatore comprende entrambe le correzioni e il dettaglio delle spiegazioni delle domande: 95 test passati e release GNU riuscita. Questo esito non certifica l'intero prodotto o la sicurezza dei componenti esterni.

Non sono stati avviati GUI, chiamate, acquisizioni, scanner, connessioni al daemon, letture di chat reali o riproduzioni di attacco dal revisore. La compatibilità effettiva con una specifica build alpha di Codex e l'esperienza Windows rimangono da verificare separatamente.

## Copertura: bridge e risposte umane

| Superficie | Percorso letto e conclusione statica |
| --- | --- |
| Avvio e autorità locale | `src/codex_bridge.rs:88`, `src/codex_bridge/transport.rs:64`: avvio solo richiesto dal controller, socket assoluto locale, rifiuto UNC, CLI lanciata con argomenti strutturati e senza shell. Viene creato solo `app-server proxy --sock`; il bridge non avvia un nuovo daemon/agente. |
| Dati privati | Il bridge usa metadati filesystem per scoprire socket/eseguibile e risposte dell'endpoint. Non legge auth, cookie, database o file di conversazione. La fiducia del socket/CLI locale deriva dall'account Windows e dalla sua configurazione; non è una nuova autenticazione crittografica implementata dall'app. |
| Domande | `src/codex_bridge.rs:118`, `src/codex_bridge/protocol.rs:191,233`: identità epoch/requestId/thread/turn ricontrollata nel worker contro il prompt corrente. Risposta consumata una volta; resolved, fine turno e chiusura chat invalidano i prompt. |
| Permessi | Il bridge non genera grant automatici né esegue RPC tool. Le approvazioni vengono mostrate per gestione in Codex. L'API di rifiuto consente solo decline per tipi supportati e ricontrolla la chiave. |
| Campi riservati | `src/assist.rs:286`: le domande con is_secret vengono rinviate a Codex, senza form compilabile nel pannello. |
| Messaggio successivo | `src/codex_bridge/protocol.rs:249`: soltanto thread/queue/add seguito da thread/queue/start con ID restituito. Una guardia per thread copre sia Enqueue sia StartQueued, fino ad ACK/errore finale; un secondo clic nel frattempo non crea un altro messaggio. Nessuna deduplicazione permanente per testo. Nessun turn/start, steer o interrupt. Lo stato Idle è ricontrollato nel worker; l'upstream applica inoltre StartIfIdle atomico. |
| Limiti | 32 chat già caricate, 32 prompt, 128 richieste pendenti, 2 KiB di anteprima, coda comandi 16; frame/messaggi WebSocket limitati a 512 KiB. Oltre il limite wire il collegamento viene interrotto con errore. Non viene promessa la lettura di tutte le chat desktop su server incorporati non esposti. |
| Processo posseduto | `src/codex_bridge/transport.rs:116,127`, `src/codex_bridge.rs:152`: kill riguarda esclusivamente Child creato dal bridge, poi wait/join e chiusura pipe; nessun arresto del daemon condiviso. Scrittura in coda limitata, deadline 5 s e lettura non bloccante con PeekNamedPipe. |

Il join di Bridge non ha una deadline assoluta: presuppone che spawn/TerminateProcess e la chiusura delle pipe del processo posseduto terminino. Non è stata dimostrata una condizione normale di stallo; non viene quindi riportata come vulnerabilità confermata o arresto bounded del sistema operativo.

## Copertura: controller e consumo

`src/assist.rs:121` crea bridge/metriche soltanto con dashboard abilitata e overlay disponibile; il monitor metadati richiede call_panel abilitato. Disabilitazione, assenza del monitor secondario e Drop rimuovono i worker posseduti. I flag richiedono esattamente `1`; l'impostazione assente è disattivata. Tick e actions consumano il segnale di topologia prima di usare il pannello. I cambi display/DPI nascondono la finestra e rinviano l'enumerazione al controller; questi sono percorsi statici, non hotplug provato sul desktop.

`src/assist.rs:196,319` mantiene ConfirmCall in errore e can_listen/listening a false. Il catalogo delle chiamate non viene convertito in input Codex. Le risposte e i follow-up richiedono un'azione manuale e restano associati alla chat/richiesta selezionata. Il collegamento esterno codex:// accetta soltanto un identificatore nel formato UUID esadecimale previsto. I messaggi d'errore/esito del bridge hanno precedenza sulla nota locale di richiesta invio; questa nota non viene presentata come conferma del server.

Il ramo non secret di dashboard aggiunge header/testo e descrizioni delle opzioni al dettaglio scorrevole, mantenendo le etichette originali come valori delle combo. Il ramo secret esclude anche questo nuovo dettaglio della richiesta. Le stringhe restano soggette ai limiti del parser e del modello UI; nessun permesso o IPC aggiunto. Il nuovo test sintetico verifica descrizione leggibile, label originale e form assente per is_secret.

`src/assist_metrics.rs:56,104,122`: campionamento ogni due secondi, coda risultati di capacità uno, fino a 256 processi candidati, handle con proprietario RAII. Le CPU delta distinguono PID e FILETIME di creazione; primo campione/reset restano sconosciuti. Le etichette dichiarano totali di processo condivisi, senza attribuzione per chat. La selezione dei processi Codex è per nome eseguibile: non costituisce un'identità attestata. Nessuna lettura del contenuto della memoria dei processi.

Il rilascio del worker metriche attende al massimo circa 200 ms, quello metadati circa 900 ms. Una chiamata OS già in corso può sopravvivere a quell'attesa; gli oggetti del worker ne mantengono valida la memoria fino al ritorno. Non sono misure di costo RAM/CPU o garanzie di rilascio kernel entro quella scadenza.

## Copertura: metadati audio

`src/call_audio.rs:48,70` lega la candidata a PID, FILETIME di creazione e sessionInstanceIdentifier; una diversa incarnazione del processo riceve una chiave nuova. Il suggerimento richiede correlazione esatta con la notifica communications, non il semplice fatto che un processo riproduca musica. Una notifica anticipata viene conservata per una breve finestra; un ID sconosciuto scade.

`src/call_audio/windows.rs:109,113` esclude i suoni di sistema e i successi GetProcessId multiprocesso ambigui, preservando l'HRESULT originale. Le registrazioni riguardano endpoint render, non capture; non vengono aperti client audio o microfoni. I callback copiano stringhe limitate, accodano senza attendere e segnalano il worker MTA. Unregister e release finali avvengono fuori dai callback, prima di CoUninitialize. Le stringhe restituite con proprietà COM vengono liberate con CoTaskMemFree.

Budget: massimo 16 endpoint, 128 registrazioni di sessione/candidate, 32 eventi in coda e 32 eventi elaborati per ciclo, ID fino a 1024 unità UTF-16. Overflow azzera la correlazione ed espone errore. `src/call_audio.rs:93,96` conferma sempre fallita e poll_audio sempre vuoto: nessuno stato Listening o AudioChunk viene prodotto.

Il ducking è un indizio facoltativo; mixer propri e implementazioni delle applicazioni possono non produrre eventi utili. Il catalogo render non dimostra che esista una chiamata né isola la scheda del browser. Nessuna affermazione di trascrizione o compatibilità process-loopback deriva da questo codice, che non implementa acquisizione.

## Registro dei candidati e tentativi di confutazione

| ID | Stato | Caso e controverifica |
| --- | --- | --- |
| A7-1 | Confermato nel sorgente, corretto | Un produttore continuo poteva riempire la coda mentre il worker eseguiva while try_recv, prolungando il drain prima del controllo stop/refresh. La capacità 32 limita lo spazio ma non il lavoro di un drain concorrente. Ora il for elabora al massimo MAX_EVENTS per ciclo (`windows.rs:192`). Nessuna saturazione reale o riproduzione di attacco eseguita. |
| A7-2 | Rigettato per fonte upstream esaminata | Possibile riuso requestId per nuova domanda nello stesso turn, con vecchia form ancora aperta. Controevidenza: outgoing_message.rs:493-494 assegna AtomicI64.fetch_add a ogni richiesta; replay mantiene il callback originale. Restart richiede nuova connessione/epoch. Non dimostra il comportamento di ogni versione binaria futura. |
| A7-3 | Rigettato | Un follow-up osservato Idle potrebbe raggiungere il server dopo che la chat è divenuta Active. Controevidenza: thread_queue_processor.rs:217-234 gestisce StartIfIdle e rifiuta NotIdle/PendingTriggerTurn; l'errore lascia il messaggio in coda senza ritentarlo automaticamente. |
| A7-4 | Rigettato | Un campo secret potrebbe comparire come testo in una combo ordinaria. Controevidenza: il controller non crea QuestionForm quando qualunque domanda del gruppo è is_secret e rimanda a Codex. |
| A7-5 | Segnalato dal coordinatore, corretto e riletto | Due clic mentre la chat risulta ancora Idle potevano produrre due queue/add. Pending::StartQueued ora conserva il thread; la guardia in command rifiuta ulteriori follow-up durante Enqueue e StartQueued. ACK/errore libera la guardia; chiusura/unload rimuove anche gli RPC ancora non inviati. Risposte tardive a pending rimossi vengono ignorate. I due test sintetici aggiunti verificano entrambi gli stadi, rilascio su errore/chiusura e la possibilità di inviare intenzionalmente lo stesso testo in seguito. Nessun doppio invio live riprodotto. |

Le due controverifiche server usano copie della fonte pubblica ufficiale sotto work/iteration-7/openai-reference, revisione upstream af719423 individuata dal coordinatore. Sono evidenze statiche del protocollo, non accessi a conversazioni dell'utente.

## Verifiche e residui

Il coordinatore ha eseguito e i log locali finali confermano 95 test passati, zero falliti, in 0,48 s e release GNU v0.1.7 riuscita in 25,70 s, dopo A7-1, A7-5, gli ultimi aggiustamenti della topologia UI e le spiegazioni delle domande. Questi controlli sono del coordinatore, non eseguiti dal revisore. I quattro test call_audio usano solo identità/eventi sintetici e non attivano COM o acquisizione. Un'eventuale CI Windows successiva ha evidenza distinta; qui non se ne anticipa l'esito.

Il coordinatore ha eseguito inoltre il comando diagnostico --codex-check: il log registra endpoint condiviso non disponibile nell'attuale installazione desktop. `src/main.rs:33` tratta il comando prima di impostazioni, COM, mutex o creazione finestre e ritorna subito; `check_local` esegue soltanto discovery di metadati, senza proxy o RPC. Questo esito negativo non verifica l'API di una chat attiva, né il collegamento reale dell'installazione presente.

Restano non provati: GUI/DPI/multi-monitor/accessibilità e tastiera reale, collegamento al daemon e compatibilità della build CLI installata, chat e richieste live, lifecycle dei processi nel sistema operativo, rilevamento di chiamate reali, RAM/CPU/latenza del nuovo scenario. Audio AI e PCM sono intenzionalmente assenti dalla versione richiesta. Nessun test di exploit o scanner avviato.

## Impronte delle fonti lette

SHA-256 rilevati dopo A7-1, A7-5 e la rilettura finale. Di assist_overlay sono stati letti i chiamanti e i percorsi form/topologia pertinenti a questo audit, non rivalutato l'intero rendering Win32. Gli eventuali cambi successivi richiedono rilettura del relativo perimetro.

| File | SHA-256 |
| --- | --- |
| src/codex_bridge.rs | 05ed85e87fa3e9044c3a7604e1cae5b14986ee9e2a0bb95f17d3900fe59c95da |
| src/codex_bridge/protocol.rs | 656844ea8abdc9d426207e1797770fd8c91d377cec140a16c6e8b46e324971b6 |
| src/codex_bridge/transport.rs | c6b69a62a0c31f2b5ff80062a68fb4d3bb61daca2a8398dd8e24fd809f3fd221 |
| src/assist.rs | 807d25abe80a87ba3ecf8d6f0ddd5df4725d93c08efcdd8ddf2b6d069a85aa38 |
| src/assist_metrics.rs | f98a4d44027b50cf6c135ed182fc842a5c54c7f52ed91fcdc70d5c7f72abf302 |
| src/assist_overlay.rs | 20ce47ec26db85d6328799aa210cc51e5ea056b8872a2d112ed2ebca43b5bda9 |
| src/call_audio.rs | eb0b63117c58c97b085dfdf7d3ac77104e820b6c2654a705956201d44f35d371 |
| src/call_audio/windows.rs | 667f8012165641861d333abfb06c523befe1006b299a97d4b119706d375b8d2c |

Verdetto: nessun blocco statico aperto nel perimetro della preview v0.1.7, con validazione CLI finale del coordinatore completata e limiti runtime sopra conservati.
