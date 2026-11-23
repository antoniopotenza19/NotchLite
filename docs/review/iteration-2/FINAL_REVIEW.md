# Revisione indipendente — iterazione 2

What this change does: l'app apre il pannello reale quando viene avviata normalmente e resta come capsula con --background. Questa iterazione aggiunge incolla esplicito negli editor, sospensione del rendering nascosto e inizializzazione del profilo tramite file temporaneo completo, senza sostituire preferenze già esistenti.

La revisione riguarda NotchLite e il suo team di sviluppo. Non valuta né realizza una personalizzazione estetica dell'intera interfaccia Windows. Carico atteso: un utente desktop, una sola istanza, moduli opzionali attivati individualmente.

## Esito

Non ho individuato una regressione bloccante dimostrata nei quattro file esaminati. Le modifiche sono proporzionate ai comportamenti richiesti; non introducono framework o dipendenze runtime per Graphify/Ponytail. Il team resta collegato alle istruzioni di progetto.

Il verdetto resta quello di un'anteprima sperimentale. Avvio del processo, test automatici, misure RAM e controllo visivo sono evidenze diverse: questo revisore ha lavorato sui sorgenti e non ha avviato il programma o automatizzato la GUI.

## Snapshot esaminata

| File | Righe | SHA-256 |
|---|---:|---|
| src/main.rs | 39 | d846dc88685f2af8494d4ff43e9712baa89866e691d3308b1b50fc60866b963a |
| src/shell.rs | 1363 | e95350e393cefa0d85ffca6f09db30b54dc66e3187d23700067e6ea4468191fe |
| src/profile.rs | 173 | 99cbe83354d2401a544b78b286e368e7702ee07ffb45d671c44eefa391ab232c |
| src/droptarget.rs | 464 | e8d7138bdf973494733a146d8482b5a78d8c93bc6159d92df6a0d927efb864ac |

Lettura integrale dei quattro file, diff rispetto al checkout di base, chiamanti di clipboard_text, request_open_existing e shell::run, percorsi degli editor Todo/Calendar e TextField. Sono state controllate anche le relazioni tra tick/poll, Waker e worker meteo per il residuo descritto sotto.

## Controlli sulle modifiche

- **Apertura e istanza esistente.** main.rs:23-37 distingue l'apertura esplicita da --background; la richiesta alla finestra esistente non trasporta testo o percorsi utente (shell.rs:1212-1219). L'avvio automatico registra --background (shell.rs:1053-1065). Il pannello esplicito viene tenuto aperto e restituito alla capsula da Escape fuori dall'editor (shell.rs:416-472, 855-863). Taskbar, focus e riapertura tra processi restano da provare nella GUI.
- **Rendering nascosto.** guard_z_order ferma l'animazione e il timer della mascotte quando la capsula è nascosta; start_animation, step_animation e redraw rispettano quello stato (shell.rs:345-414, 487-515, 922-938). Quando torna visibile riparte da un timestamp corrente. Il tick funzionale resta attivo; la patch non deve essere descritta come consumo CPU sempre nullo.
- **Incolla.** Ctrl+V è gestito soltanto quando un editor è attivo (shell.rs:855-875). clipboard_text legge su richiesta il formato testo Unicode e limita a 512 unità UTF-16 la costruzione della stringa (droptarget.rs:297-334). Il chiamante filtra i controlli e tenta al massimo 120 caratteri; TextField conserva anche il proprio limite totale di 120, inclusi i caratteri già presenti (textfield.rs:10, 74-84). Il WM_CHAR di controllo eventualmente prodotto da Ctrl+V viene già rifiutato da TextField. Non sono introdotti cronologia, listener o scritture degli appunti.
- **Rilascio della memoria OLE.** GlobalUnlockGuard viene creato dopo Medium e quindi rilasciato prima del medium, anche nelle uscite di unwinding. La lettura rispetta la dimensione del blocco; i casi di lock fallito ritornano senza costruire una slice. Il limite di 512 riguarda la stringa letta dal helper: non è una garanzia generale sulla memoria usata da un provider OLE che materializza il contenuto completo.
- **Profilo iniziale.** profile.rs:24-100 scrive su staging riservato con create_new, sincronizza e pubblica nello stesso percorso senza REPLACE_EXISTING né COPY_ALLOWED. Errori di scrittura non pubblicano un file finale parziale; una configurazione creata nel frattempo resta prevalente. Una directory o un collegamento simbolico al posto del file atteso fallisce esplicitamente. I test presenti coprono conservazione, scrittura interrotta, creazione concorrente e percorso occupato da directory.
- **Validazione.** Nei sorgenti sono presenti 18 test unitari. Non sono stati rieseguiti da questo revisore; compilazione, suite e benchmark sono responsabilità del coordinatore. Non viene dedotto dal numero dei test che i percorsi GUI siano coperti.

## Residuo con validazione runtime mancante

### R-01 — Una wake durante il menu modale può lasciare un risultato in attesa

**Situazione concreta:** il modulo meteo è stato appena attivato, si apre il menu contestuale e il primo risultato arriva mentre TrackPopupMenu gestisce il suo ciclo modale. show_tray_menu tiene il borrow di App per tutta la chiamata (shell.rs:941-976); un messaggio WM_APP_WAKE rientrante incontra try_borrow_mut occupato e viene passato al comportamento predefinito (shell.rs:1151-1155).

**Traccia ed effetto possibile:** il worker accoda il risultato e invia una wake, poi attende 15 minuti se il fetch è riuscito (weather.rs:116-137). Il risultato viene applicato solo da poll(), mentre WeatherPage::tick non svuota il canale (weather.rs:154-182). Se la wake è stata consumata durante quel ciclo modale, la pagina può restare su Locating fino a una wake successiva. La chiusura del menu non riposta i segnali.

**Limite dell'evidenza:** la traccia sorgente e il punto di rientranza sono chiari, ma questa specifica sequenza di dispatch del menu non è stata eseguita nella GUI. Non è un exploit dimostrato, né un blocco della configurazione base con meteo disabilitato. È un comportamento ereditato, non causato dall'aggiunta dell'incolla.

**Patch minima futura:** dopo la chiusura del menu, ripostare WM_APP_WAKE e WM_APP_CMD affinché i risultati e i comandi già accodati siano drenati quando il borrow esterno è finito. Non serve introdurre polling continuo o un nuovo scheduler. La modifica non fa parte di questa iterazione.

**Prova di chiusura futura:** con risultato meteo sintetico, consegnarlo mentre il menu è aperto; dopo la chiusura, verificare che venga applicato senza attendere un nuovo fetch. La prova deve rispettare l'ambiente di esecuzione autorizzato.

## Team persistente e grafo

AGENTS.md rimanda a notchlite-development e al team; la skill di sviluppo rimanda a Ponytail Review e alle fonti. Tutti i collegamenti locali verificati risolvono. I riferimenti Ponytail e le licenze MIT corrispondono agli SHA-256 registrati in SOURCES.json. Le istruzioni restano limitate al progetto e non impongono delega ai microfix, installazioni automatiche o nuovi passaggi di approvazione.

All'inizio della revisione il grafo era obsoleto per shell.rs e droptarget.rs: è stato usato solo per orientamento. Dopo il refresh del coordinatore, SOURCE_SNAPSHOT.json corrisponde ai 24 file Rust correnti; il grafo contiene 893 nodi, 2.105 archi e 22 comunità. Le relazioni AST non sostituiscono il controllo dei chiamanti o una prova runtime.

La cache tecnica Graphify è esclusa da Git. Nel controllo dell'indice non sono comparsi file saved/reel/Instagram. Nessun contenuto degli appunti, credenziale o dato di navigazione è stato letto da questo revisore o incluso nel report.

**Verdict: Ship come anteprima sperimentale.**

**Not checked:** GUI, incolla reale da provider OLE, apertura da seconda istanza, focus/taskbar, Escape, fullscreen reale, DPI/monitor multipli, comportamento dopo crash di processo e valori RAM/CPU. Le misure del coordinatore vanno riportate con il loro profilo e scenario.
