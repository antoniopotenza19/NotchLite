# Revisione indipendente — sessione LAN nativa 0.1.6

Data: 2026-11-23. Revisore: agente security_audit, distinto dagli autori del backend, della UI e delle companion. Metodo: AGENTS.md, Ponytail Review e adattamento APTIOS security-review. Responsabilità di scrittura limitata a questo report. Sono stati letti sorgenti, chiamanti, test e documentazione ufficiale; il revisore non ha eseguito app, socket, scanner, build o riproduzioni di attacchi. Questa è una revisione statica della nuova funzione, non un audit completo del prodotto o delle dipendenze.

**Cosa cambia:** un interruttore inizialmente spento abilita una pagina Phone. Il listener TLS parte soltanto dopo un'azione esplicita. Un QR temporaneo, il confronto di un codice e l'approvazione sul PC autorizzano una singola sessione volatile; i file richiedono offerte esplicite e limiti di dimensione.

**Esito del perimetro statico:** dopo le correzioni sotto descritte non sono emersi ulteriori blocchi concreti nei percorsi ispezionati. Non viene certificata la sicurezza dell'intero prodotto, né il funzionamento delle app su Android/iPhone. La compilazione e le prove con telefoni reali restano evidenze distinte.

## Modello di fiducia e risorse protette

Un altro host della LAN può aprire connessioni al listener, ma non deve leggere file o autorizzarsi per il solo fatto di conoscere IP e porta. Il QR contiene un segreto monouso casuale di 256 bit e l'impronta SHA-256 DER del certificato PC; la companion deve verificare l'identità TLS prima di inviare il segreto. Il possesso del QR ammette solo una richiesta in attesa: il PC autorizza il suo ID esatto dopo il confronto del codice.

`pollToken` e token di sessione hanno ruoli distinti. Il primo permette solo di interrogare quella richiesta, il secondo autorizza gli endpoint della sessione approvata. Si tratta di bearer token in memoria, senza mTLS, chiavi persistenti del telefono o ripristino dopo un nuovo avvio. Non è il precedente progetto di trasferimento via browser; non introduce Bluetooth, shell remota o esplorazione arbitraria del filesystem.

La revisione assume sistema operativo e processo dell'utente non compromessi. Verifica il confine verso input di rete e metadati dei file, senza attribuire al programma protezione contro un amministratore o malware già nello stesso processo.

## Registro di copertura

Le righe si riferiscono agli hash riportati in fondo. Il grafo è un ausilio: la verifica dei confini è stata svolta sui sorgenti e sui chiamanti effettivi.

| ID / superficie | Principale e risorsa | Sorgenti / controlli | Stato e residuo |
|---|---|---|---|
| C1 attivazione | Utente PC; listener, disco, thread | `src/pages/mod.rs:177`, Settings e `shell.rs` rebuild guard; `pages/phone.rs:34`, `:84`. Flag esatto `1`, costruzione inerte, avvio esplicito, rollback se QR fallisce. | Ispezionato staticamente; nessuna misura idle. |
| C2 identità TLS | Peer LAN; segreto e bearer | `src/phone.rs:130`, `:335`; Android `ApiClient.kt:24`; iOS `PinnedClient.swift:54`. Identità temporanea, pin DER, hostname/policy, redirect disabilitati. | Ispezionato staticamente; trust reale dei due sistemi mobile non provato. |
| C3 associazione | Detentore QR; autorizzazione della sessione | `phone/wire.rs:129`, `:149`; `phone.rs:181`. Segreto monouso, token separati, scadenza monotona iniziale, approvazione ID/status/deadline. | Ispezionato; nessun pairing reale del revisore. |
| C4 HTTP e carico | Peer LAN non autenticato; memoria e worker | `phone/wire.rs:12`, `:55`, `:94`, `:121`; `phone.rs:287`. Header 16 KiB/48 campi, JSON 8 KiB, CL non ambiguo, no TE/Origin/Expect/Range, quattro worker, deadline assolute. | Ispezionato; carico e rete ostile non riprodotti. |
| C5 file in entrata | Telefono autorizzato; filesystem PC | `phone/wire.rs:163`, `:235`, `:299`; `phone.rs:353`, `:366`, `:386`. Consenso per ID, claim atomico, nome singolo, no device/ADS/traversal, guardie directory, staging posseduto e publish senza overwrite. | Ispezionato; guasti disco/kernel e tutti i filesystem non provati. |
| C6 file in uscita | Telefono autorizzato; solo file offerto dal PC | `phone.rs:214`; `phone/wire.rs:338`, `:204`. Handle del file scelto, dimensione da quell'handle, condivisione Windows senza write/delete, claim esclusivo, retry Complete e ACK esplicito. | Ispezionato; nessuna lettura di file utente da parte del revisore. |
| C7 stop e cancellazione | Utente PC/telefono; sessione e trasferimento esatti | `phone.rs:230`, `:245`, `:269`, `:287`; `phone/wire.rs:15`, `:276`. Revoca atomica, chiusura socket, cancel per ID e controllo prima del publish. | Ispezionato; syscall disco già in corso può finire dopo Stop, vedi residui. |
| C8 UI/QR | Utente PC; approvazione e lifetime Win32 | `pages/phone.rs:56`, `:64`, `:163`; `phone_qr.rs:40`, `:82`, `:132`. Box stabile, USERDATA preso in prestito, destroyed flag, paint accoppiato e chiusura gestita. | Ispezionato; scanner, focus, DPI e GDI reali non provati. |
| C9 companion | Utente mobile; credenziali, stato e destinazione scelta | Android PairQr/ApiClient/MainActivity; iOS PairQR/PinnedClient/CompanionModel. Parsing QR stretto, transfer delimitato, annullamento, guardie generazione e cleanup. | Logica critica ispezionata; build mobile e dispositivi non validati dal revisore. UI mobile completa, entitlement e store non certificati. |
| C10 dipendenze/build | Autore/build; catena di distribuzione | Cargo.toml/lock letti: rustls 0.23.46, ring 0.17.14, rcgen 0.14.10, httparse 1.10.1, qrcodegen 1.8.0. | Parziale: nessun audit interno di librerie, toolchain, CI o distribuzione firmata. Nessuno scanner avviato. |

## Problemi confermati nel sorgente precedente e chiusi

Questi record distinguono il difetto osservabile dal metodo di verifica. La chiusura qui significa che la correzione è stata riletta; non significa che il revisore abbia eseguito un exploit o un dispositivo. Alcuni problemi sono stati individuati anche dal coordinatore o dall'autore e verificati separatamente in questa revisione.

### R1 — listener terminato con stato ancora disponibile

**Stato: confirmed, corretto.** Un errore di accept usciva dal loop scrivendo solo `listener_failed`; token e segreto restavano presenti e lo snapshot poteva continuare a dichiarare disponibilità. Controevidenza cercata: non esisteva una verifica dell'esito del worker nello snapshot. Risorsa compromessa: correttezza dello stato di autorizzazione mostrato, non lettura remota di file dimostrata.

Ora il finale di `run` (`phone.rs:321`) chiude listener/socket, marca stopped e revoca le credenziali. `snapshot_state` (`:276`) condiziona lo stato a stopped. Validazione del revisore: trace statico, senza indurre errori di rete.

### R2 — polling Android interrotto dopo un trasferimento

**Stato: confirmed, corretto.** Il job di upload/download invocava `poll()` prima di terminare; `poll` vedeva lo stesso job ancora attivo e non ripartiva. Dopo il primo invio le nuove offerte PC potevano quindi non comparire. Controevidenza: non c'era un altro timer di polling che riavviasse il percorso, salvo un successivo evento lifecycle.

`MainActivity.kt:95` ora libera job e stato busy nel finally della generazione corrente, poi riavvia il polling. Anche la fine del vecchio poll è condizionata alla generazione (`:124`). Controllo statico; nessuna interazione Android eseguita.

### R3 — conferma del salvataggio incompatibile tra client e server

**Stato: confirmed, corretto.** Android inviava `{}` sul POST done mentre il server accettava solo Content-Length zero: il file poteva essere salvato ma la conferma falliva sempre. Non era risolto dal retry manuale, che inviava lo stesso corpo.

Le companion ora emettono esplicitamente CL0 sul POST senza corpo; `phone/wire.rs:204` accetta anche un oggetto JSON vuoto entro i limiti, rifiutando altri contenuti. Il test funzionale del backend include la variante `{}`; il codice mobile è stato riletto. L'ACK significa conferma applicativa del salvataggio, non garanzia remota di fsync del provider mobile.

### R4 — comando UI in attesa del mutex durante publish disco

**Stato: confirmed sul percorso di contesa, corretto.** Lo snapshot/Stop erano stati resi non bloccanti, ma Cancel e altri comandi UI usavano ancora il lock bloccante condiviso con MoveFileEx. Un publish fermo nel sistema poteva bloccare anche il clic di annullamento. Controevidenza: il fatto che lo snapshot usasse try_lock non proteggeva quei chiamanti.

I comandi UI (`phone.rs:181`, `:198`, `:204`, `:222`, `:230`) usano ora try_lock e restituiscono busy/retry. Stop invalida l'autorizzazione tramite flag atomico e limita l'attesa sul worker (`:245`). Non è stata simulata una syscall bloccata; il limite delle operazioni già entrate nel kernel rimane esplicito.

### R5 — risultati asincroni e credenziali di una sessione precedente

**Stato: confirmed source-supported, corretto; interleaving non riprodotto su dispositivo.** Il solo cancel del task non impediva a un risultato già completato di riprendere l'azione iOS, modificando token/stato dopo Disconnect. Inoltre un vecchio poll poteva azzerare il riferimento a un task nuovo. La cancellazione Swift è cooperativa: un controllo nel solo defer non protegge le mutazioni dopo await. [Documentazione Swift](https://docs.swift.org/latest/documentation/swift/task/cancel%28%29/).

La guardia ora è all'ingresso e dopo gli await, con generazione verificata prima di esporre risultati (`CompanionModel.swift:48`, `:68`, `:75`, `:105`, `:149`; `PinnedClient.swift:113`). Il cleanup del trasporto usa identità `active === handle`. Progress di vecchi task non aggiorna la nuova UI. Android cattura client, ID e bearer sul thread UI prima dell'I/O (`MainActivity.kt:173`, `:184`), evitando letture intercalabili dei tre campi globali durante un cambio sessione. Sono stati verificati i chiamanti reali e il binding del client al suo QR; nessuna fuga di token è stata osservata o riprodotta.

### Ricontrollo finale dei callback dei selettori file

Riletto il diff mirato rispetto a `49f064`: Android `onStart/work/disconnect` (`MainActivity.kt:78`, `:95`, `:207`) e iOS `scene/work/disconnect` (`CompanionModel.swift:36`, `:48`, `:42`) conservano al massimo una nuova azione manuale ricevuta mentre l'app è inattiva. L'azione non avvia rete in quello stato; al ritorno in primo piano viene estratta e azzerata prima di passare attraverso le normali guardie di work. Disconnect elimina la coda. Le task cancellate non vengono inserite automaticamente nella coda e non viene ripreso un trasferimento interrotto.

Il cleanup iOS del file da esportare rimuove anche la sua directory temporanea: il relativo URL proviene dalla directory UUID creata dal modello, non dal picker o da un path ricevuto dal PC. `PinnedClient.swift:126` usa ora withLock per brevi accessi sincroni, senza await sotto lock, conservando il confronto di identità nel defer. L'annotazione esplicita `(Data, URL?)` non cambia il controllo di autorizzazione. Nessun nuovo difetto concreto emerso in questo diff; ordine reale dei callback e compatibilità di compilazione restano da verificare in CI/dispositivo. Gli hash dei tre file sono aggiornati sotto; backend Rust invariato.

### Ricontrollo finale dell'avvio URLSession e della cancellazione

**Ipotesi di crash per task creato su sessione già invalidata: rejected sul sorgente precedente ispezionato.** Quella versione chiamava `transport.start` prima di `handle.install`; cancel non poteva ottenere il Transport finché la creazione/resume del task non era già terminata. La controevidenza impedisce di attribuire a quella fonte un crash per l'ordine inverso. Rimaneva però una finestra nella quale una cancellazione già richiesta poteva essere seguita da un breve avvio, poi interrotto all'installazione dell'handle.

L'hardening finale `CancellationHandle.installAndStart` (`PinnedClient.swift:17`) controlla il flag e serializza installazione, creazione e resume con lo stesso lock di cancel. Se la cancellazione ha già vinto non viene creato alcun task; il chiamante, fuori dal lock, invalida la sessione vuota e completa manualmente la continuation con CancellationError. Se l'avvio ha vinto, la creazione termina prima dell'invalidazione e soltanto il callback didComplete nativo completa la continuation. Sono stati controllati entrambi i percorsi e il controllo post-await già presente; nessuna continuation viene completata dentro il nuovo lock.

[Apple vieta il riuso di una URLSession invalidata](https://developer.apple.com/documentation/foundation/urlsession/invalidateandcancel%28%29) e precisa che [non possono essere creati nuovi task dopo l'invalidazione](https://developer.apple.com/documentation/foundation/urlsession/finishtasksandinvalidate%28%29). Queste fonti sostengono l'invariante, non dimostrano un NSGenericException su questa app. Nessun crash, test runtime o connessione è stato eseguito dal revisore per questo ricontrollo; la compilazione CI resta una verifica distinta. Solo l'hash di PinnedClient cambia in questo passaggio.

### R6 — approvazione dopo i due minuti iniziali

**Stato: confirmed rispetto al contratto finale, corretto.** Un POST vicino alla scadenza assegnava al nuovo PairAttempt altri 120 secondi, estendendo la finestra totale. Il token era ancora condizionato al consenso PC, quindi non era un bypass del consenso; era una scadenza diversa da quella promessa.

`phone/wire.rs:143` riusa ora `secret_deadline` iniziale; il consenso rilegge deadline e stato pending sotto lock (`phone.rs:181`). Un ID non corrente o già scaduto non viene promosso. Il revisore ha riletto la correzione e i controlli sintetici dichiarati dagli autori.

### R7 — percorsi errore della finestra QR

**Stato: confirmed come gestione API incompleta, corretto.** Il ramo HDC invalido usciva senza EndPaint; NCCREATE non rilevava un fallimento nell'installazione di USERDATA, lasciando possibile una finestra accettata senza stato disegnabile. Nessun crash o exploit è stato riprodotto.

`phone_qr.rs:85` accoppia EndPaint anche sul ramo d'errore; `:133` usa SetLastError/esito di SetWindowLongPtr e rifiuta la creazione fallita. Il Box rimane proprietà di QrWindow fino a DestroyWindow conclusa; WM_NCDESTROY marca destroyed e cancella il puntatore preso in prestito.

## Altri controlli e ipotesi confutate

- **Cancellazione del socket:** riletta la correzione dell'autore che restituisce ConnectionAborted, non Interrupted, dai controlli del socket (`wire.rs:15`). Gli helper standard di read/write possono ritentare Interrupted; il flag del singolo trasferimento ora viene controllato anche durante l'attesa sul socket, non soltanto fra chunk. Il fallimento di set_read/write_timeout termina prima dell'handshake (`:94`).
- **Percorso remoto arbitrario: rejected.** Il client non fornisce il path da aprire. Per l'ingresso il suo nome viene validato, l'ID della richiesta deve coincidere con un'offerta generata dal server e il publish avviene in una sottodirectory controllata. Per l'uscita viene clonato l'handle scelto dal PC. Il nome Windows rifiuta anche ADS, trailing dot/space e COM/LPT con cifre apicali; queste ultime sono nomi riservati documentati da [Microsoft](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file).
- **Overwrite e cleanup di file altrui: rejected nel percorso finale ispezionato.** Staging usa create_new e registra owned solo dopo il successo; Drop rimuove solo il proprio payload non committed. MoveFileEx usa WRITE_THROUGH senza REPLACE_EXISTING/COPY_ALLOWED. Le directory ricevono guardie contro reparse e rename; un file completo non viene cancellato dal cancel di un trasferimento successivo.
- **Seconda richiesta nascosta dopo Content-Length: rejected come request smuggling in questo server.** La copia scrive esattamente la lunghezza approvata e rifiuta troncamenti; eventuali byte oltre il framing non diventano un altro comando perché si serve una sola richiesta e si chiude la connessione. Non è una dichiarazione di compatibilità con proxy o keep-alive, che qui non sono implementati.
- **Trust globale allentato su iOS: rejected.** Il pin deve corrispondere al leaf, usato come sola anchor del particolare oggetto SecTrust con policy SSL per l'host e valutazione piattaforma, senza eccezioni per scadenza o fetch di rete. Non si installa una CA nel dispositivo: [Apple specifica che queste modifiche riguardano soltanto il trust referenziato](https://developer.apple.com/documentation/security/configuring-a-trust). Il successo effettivo e il rifiuto di certificati scaduti/hostname errato su iPhone rimangono da provare.
- **Certificato temporaneo:** `generate_identity` (`phone.rs:335`) limita la validità a circa 24 ore con cinque minuti di tolleranza iniziale, IP SAN, EKU ServerAuth e DigitalSignature. Il pin è calcolato sul DER dello stesso certificato servito. La modifica evita di affidarsi ai lunghi valori predefiniti del generatore; non sostituisce una prova con Security.framework o Android.

## Evidenze eseguite da altri e limiti residui

Il coordinatore riporta **59 test Rust passati, zero falliti**, e build release riuscita con `cargo test --locked --offline` e `cargo build --release --locked --offline`. Il revisore ha letto le righe finali dei log locali dell'iterazione: test in 0,47 secondi e release conclusa in 18,46 secondi. Il record `VALIDATION.json` è del coordinatore; eventuali esecuzioni successive devono restare attribuite a chi le esegue.

La suite comprende cinque prove funzionali con TLS su loopback e porta effimera: consenso/identità, scadenza/rifiuto, upload approvato/cancel, retry/ACK in uscita, interruzione di upload parziale e Stop durante handshake inattivo. È evidenza del backend in quella fixture, non una prova Android/iPhone o una riproduzione di attacco del revisore. Nessun servizio LAN di produzione è stato avviato per questa revisione.

Residui e fatti mancanti:

1. Build mobile, firma/installazione, scanner, API di trust reali, permessi LAN e scambio bidirezionale sui dispositivi non sono stati validati dal revisore. Una build simulatore non dimostra installabilità su iPhone. Gli eventuali risultati CI successivi devono essere riportati separatamente.
2. La sessione approvata dura fino a Stop/uscita; chiudere una connessione HTTP non la revoca e non esiste prova di presenza continua del telefono. La UI usa quindi “authorized”, senza promettere heartbeat. L'identità TLS temporanea ha inoltre la propria scadenza: dopo la sua scadenza è necessario avviare una nuova sessione.
3. Stop impedisce nuove operazioni, chiude socket e limita l'attesa UI. Un I/O disco già entrato nel sistema può sopravvivere all'attesa di 900 ms, mantenendo vivi Arc/guardie, e una rinomina già avviata può concludersi dopo Stop. Non c'è garanzia di immediato rilascio di ogni worker/risorsa kernel. Cleanup è un tentativo soggetto agli errori del filesystem; file completi già ricevuti vengono conservati.
4. Firewall, VPN/multi-adapter, sospensione/cambio rete, disco pieno/errori, filesystem diversi, leak di lunga durata e resa Win32/DPI non sono stati provati. I limiti di quattro connessioni riguardano il singolo servizio; non dimostrano un limite globale a risorse trattenute da syscall bloccate di sessioni precedenti.
5. Nessuna misura RAM/CPU/GPU/latency nuova è stata eseguita. “Default off” e “lazy” sono caratteristiche del percorso sorgente, non valori di prestazione misurati.

## Identificazione dei sorgenti riletti

SHA-256 dei byte locali alla chiusura di questa revisione. File successivamente cambiati richiedono una nuova verifica mirata; conversioni di fine riga possono modificare gli hash senza cambiare la logica.

| File | SHA-256 |
|---|---|
| `src/phone.rs` | `69c9b3f69b09ccfa15736b0de0c957872a8185bcc73218f6501fa883d0bd193e` |
| `src/phone/wire.rs` | `5782749a71d89c1a05df996ad42e4928c9eb0db8d03a33845756a3ca2c8cfbd3` |
| `src/pages/phone.rs` | `97a1b4de6311d7313d8f89e2cab4d93a1ef84fb940b8799bd90f89187be15ff5` |
| `src/phone_qr.rs` | `71e3fe501986de684254e8572e5246d26135abe7360ff6cc7a07203a5961ca94` |
| Android `PairQr.kt` | `c88d023caf841502dc62f8d710bb61b5cd7b9e897a07d58f90bada6c29e8e3af` |
| Android `ApiClient.kt` | `dab651c999c2c1f6035cebdf9be5166231ed70b7cf6887519444a454bef694e9` |
| Android `MainActivity.kt` | `8e74dcbbc066b3fd6afb27d302859689599bfae5eb86bc2cf667db8663f44ded` |
| iOS `PairQR.swift` | `a9da779f2f8ae882835b4ed292c1345c8314334274584cc55f20fae8e1cd129a` |
| iOS `PinnedClient.swift` | `bacf3dc48079476645fa573f6e9bcabdc091235b1d98918d41f1e9235f693006` |
| iOS `CompanionModel.swift` | `148c83cfa14b1916f1c43f867e4f1ed3f6a7e1105635184d0ac145389335ef3e` |
| `Cargo.toml` | `b017f0d3d589e64737b7c55676de5a704f24d1dc16ee6b8344241a23604e04fb` |
| `Cargo.lock` | `ef7f42901ba5952e12eb532f241f3ae9fc2dce09de71c249b142d24319563b46` |

**Verdetto:** nessun ulteriore blocco statico concreto individuato nel perimetro descritto. La funzione rimane una preview opt-in con verifiche mobile, GUI e prestazioni ancora necessarie; questo report non costituisce una certificazione di sicurezza o di interoperabilità sui dispositivi.
