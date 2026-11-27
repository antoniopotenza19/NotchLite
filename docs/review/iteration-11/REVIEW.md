# Revisione indipendente — monitor, pannelli call e suggerimenti

Stato: revisione indipendente conclusa sul congelamento finale. I tre
rilievi riportati sono stati corretti e riletti; non restano difetti
concreti aperti nel perimetro esaminato.

Verdetto: adatto alla preview sperimentale, con i limiti runtime e di
piattaforma descritti sotto. Non è una certificazione generale di sicurezza
o di invisibilità durante una condivisione.

## Perimetro e metodo

La modifica aggiunge una preferenza nativa per il monitor della notch,
riserva ai pannelli assistente lo spazio sotto l'intera notch, aumenta
la leggibilità del pannello call e aggiorna il testo seguendo la coda
soltanto quando il lettore si trova già in fondo.

La verifica segue Ponytail Review e comprende sorgenti, chiamanti e test,
con carico atteso di un utente Windows, una notch e due pannelli nativi.
Il revisore modifica solo questo rapporto. Non avvia desktop, GUI, audio,
catture o condivisioni. La policy di esclusione dalle catture già recensita
nell'iterazione 10 non è oggetto di una nuova certificazione visiva.

## UI e monitor

- `notch_monitor` legge l'area utilizzabile e le coordinate native dei
  monitor, compresi gli schermi con origine negativa. La posizione è
  centrata in alto, senza offset fisso. La preferenza viene mantenuta
  quando il dispositivo manca o è troppo piccolo; se possibile si usa
  temporaneamente il primario. Se nessuna posizione contiene la finestra,
  viene restituito un errore senza salvare coordinate tagliate.
- Il click in Settings legge una mappa fresca, preserva le altre chiavi,
  salva la scelta e accoda ReloadSettings. Il main aggiorna la propria
  posizione prima del sync dei pannelli. Gli eventi di cambio topologia
  ripetono il posizionamento e la verifica degli overlay.
- La fascia riservata usa `GetWindowRect` della finestra principale
  appartenente al processo, non la regione animata della capsula. Nel caso
  portrait `(-1080,0,1080x1920)` con main `(-860,0,-220,190)`, il calcolo a
  96 DPI colloca entrambi i pannelli da `y=206`, con margine di 16 pixel.
  Non riserva la fascia su un altro monitor o per il solo contatto al bordo.
- Il layout dei controlli call usa rettangoli distinti per stato,
  selezione, contesto, etichette, trascrizione, suggerimento e quattro
  azioni. I caratteri della call passano a 15 DIP. I test verificano
  geometria e separazione; non dimostrano che il testo nativo sia stato
  osservato su uno schermo reale.
- L'aggiornamento di un testo call controlla lo stato della barra di
  scorrimento prima di sostituire il contenuto: segue il fondo se il
  lettore era già in fondo e preserva la prima riga altrimenti. Il campo
  dettaglio della dashboard conserva il comportamento senza inseguimento.

Non sono emersi rilievi concreti aperti nella prima lettura di questo
perimetro. La gestione completa di DPI diversi tra monitor resta il limite
dichiarato in MONITORS.md; lo spostamento del main ne conserva la scala.

## Trascrizione locale

Il segmentatore non riconta l'overlap come parlato nuovo. Dopo un taglio
forzato conserva il flag di continuazione solo quando la coda include un
frame sopra la stessa soglia vocale usata in ingresso. Sono stati riletti
i due test di pausa dopo taglio e di nuovo «no» immediato, oltre al merge.

Il worker restituisce testo vuoto per PCM interamente zero prima di creare
file TEMP o processi. Il relativo test usa un runtime volutamente
ineseguibile. I limiti di coda, risultati, generazione, epoch, processo
posseduto e annullamento restano collegati agli stessi chiamanti.

Il profilo aggiornato usa Base Q5_1, decoder con contesto completo e VAD
opzionale; le installazioni precedenti senza VAD funzionano con avviso.
Il setup conserva percorsi locali e download fissati a revisioni e SHA-256,
con il modello Tiny ancora selezionabile. Le opzioni sono state confrontate
con il [CLI upstream v1.9.2](https://github.com/ggml-org/whisper.cpp/blob/v1.9.2/examples/cli/cli.cpp).

Il revisore ha letto gli artefatti sintetici dell'autore STT: tre frasi TTS
italiane totalizzano due modifiche su 17 parole per Base/VAD, il silenzio e
il click non producono testo e i campioni brevi «sì»/«no» vengono riconosciuti.
Il log dell'harness riporta sette test superati. Non sono esecuzioni del
revisore né accuratezza misurata su Discord. Nessun consumo RAM del vecchio
profilo Tiny viene attribuito al nuovo Base.

## Rilievi e verifica delle correzioni

1. **Una risposta AI valida può scadere durante la preparazione.**
   Riferimenti finali: `src/call_coach.rs:112`, chiamante a riga 167 e
   regressione a riga 363.
   `Coach::publish` emette il cursore mentre il worker AI può ancora
   preparare il seed. Le attese ammesse per bootstrap e campione superano
   insieme i 60 secondi accettati da `State::suggest`. Una prima frase
   seguita da una preparazione lenta può quindi ottenere una risposta
   valida che `take_suggestion` scarta, pur mostrando lo stato aggiornato.
   La correzione introduce `automatic_suggestion` separato dai cursori
   manuali, mantenendo il vincolo alla sequenza, al worker della stessa
   call e alla dimensione del testo. Il test ora scade un cursore manuale,
   accetta la risposta del worker e rifiuta replay e sequenze future.
   **Chiuso nella rilettura della correzione.**

2. **L'overlap solo silenzioso può cancellare una nuova parola ripetuta.**
   Riferimenti finali: `src/call_session.rs:44`, regressione a riga 208.
   Dopo un taglio a sei secondi con 5,5 secondi di parlato e 0,5 di zero,
   il segmentatore conserva un overlap di soli zeri ma imposta ancora
   `continuation=true`. Un nuovo «no» immediato può diventare un segmento
   marcato come sovrapposto e venire eliminato se il testo precedente
   finisce per «no». Il flag ora viene mantenuto solo se la coda conservata
   contiene parlato; la regressione usa il nuovo ack subito dopo il taglio
   e verifica che `merge` conservi entrambe le parole. **Chiuso sul secondo
   freeze STT.** Il caso è stato individuato dall'autore STT e confermato
   dal revisore nei chiamanti merge.

3. **Recuperare gli ID delle risorse statiche già richieste prima di Stop.**
   Riferimenti finali: `src/call_ai.rs:98`, helper a riga 215 e test a riga 303.
   La prima implementazione interrompeva l'attesa di `thread/start` appena
   vedeva Stop, lasciando il client senza l'ID necessario per archiviare
   l'eventuale seed persistente già creato. Lo stesso bordo riguardava il
   `turn/start` statico e il suo ID per l'interruzione. Ora `drain_static_id`
   prosegue la lettura dei due ACK già inviati entro la scadenza prevista,
   registra gli ID e controlla Stop prima di proseguire. Non drena richieste
   con testo della call. Un test distingue richieste statiche già inviate
   da richieste non inviate e campioni. **Chiuso sulla correzione del client.**
   La pulizia remota resta un tentativo con timeout; non si dichiara riuscita
   se il daemon non risponde o la connessione si interrompe.

## Percorso AI riletto

Il bootstrap crea soltanto una chat tecnica posseduta con un turno statico.
Transcript e contesto arrivano esclusivamente a `thread/sample`, con prompt
UTF-8 limitato e trattato come dato non fidato. Sono disabilitati ambienti,
hook e ricerca web nel seed; una richiesta server di azione esterna provoca
errore, senza inviare risposte di approvazione. La parte manuale locale non
può chiamare l'accettazione riservata al worker automatico.

Il revisore ha confrontato il contratto col
[sorgente ufficiale fissato](https://github.com/openai/codex/blob/ad9d659a8491fc5169a00dcf7b7e6ca8a87adf69/codex-rs/app-server/src/thread_sample.rs):
il fork di campionamento intercetta la risposta prima del dispatch Core,
restituisce testo nullo per richieste di strumenti e viene ripulito dal suo
proprietario. Il modulo accetta soltanto il server verificato della serie
`0.171.0-alpha.13.2` e un account ChatGPT. La disattivazione degli hook usa
la [chiave ufficiale `hooks`](https://github.com/openai/codex/blob/ad9d659a8491fc5169a00dcf7b7e6ca8a87adf69/codex-rs/features/src/lib.rs).
`disableMcp` viene richiesto nel campione; il contratto upstream ne limita
alcuni effetti alla modalità Responses Lite e allo storico del thread.
La revisione non deduce da questo flag che tutti i server MCP configurati
dall'utente siano spenti. La barriera alle azioni è l'intercettazione prima
del dispatch, insieme alla disabilitazione della ricerca ospitata.

Le code tengono l'ultimo input e l'ultimo risultato; lunghezze di prompt,
risposta, frame e timeout sono limitati. Stop cancella subito input e
contesto condivisi, revoca i nuovi invii e attende il solo worker AI fino a
circa 900 ms; la pulizia può proseguire sul worker se il sistema ritarda.
Il seed, il turno statico e il requestId del campione sono gli unici target
di interrupt, cancel e archive. Il server condiviso e le altre chat non
vengono arrestati. Gli errori AI compaiono nello stato e non interrompono
la trascrizione locale.

## Validazione e limiti

Il coordinatore ha eseguito 156 test con 156 successi, zero fallimenti, e
build release GNU riuscita in 35,57 secondi. Il confronto dei 50 input di
compilazione prima e dopo le prove non ha rilevato modifiche. Il revisore
non ha eseguito cargo; ha letto i sorgenti, i test, gli artefatti sintetici
STT e il riepilogo finale AI, controllando i 18 hash sotto riportati.

Sul binario finale la prova AI sintetica è terminata con successo: primo
suggerimento ricevuto in circa 8,1 secondi e Stop durante il secondo
campione inviato in 111 ms. Il riepilogo letto conferma lo svuotamento dei
risultati locali e l'ACK della cancellazione da parte del server. Queste
misure appartengono a una singola prova sintetica con il servizio online;
non sono tempi garantiti per una call. Non vengono pubblicati dati di
account, piano, quota o token personali.

Il percorso produttivo di classificazione `apply_sample` è condiviso con
il test di testo nullo, oggetto e stringa vuota. La diagnostica finale usa
le stesse policy della produzione senza strumenti finti: riceve un primo
suggerimento e richiede Stop dopo aver inviato un secondo campione. Non è
una prova runtime di tool-deny. L'eventuale ACK della cancellazione è
distinto dalla cessazione effettiva di ogni elaborazione remota.

Non verificati: UI reale, focus o leggibilità osservati, scollegamento
fisico di un monitor, trascrizione di una call reale, qualità delle risposte
AI, prestazioni e RAM del prodotto con questi moduli attivi.

## Snapshot esaminato

SHA-256 del congelamento del 27 novembre 2026. Sono inclusi i chiamanti
significativi letti, oltre ai file modificati. Gli hash dei documenti non
sono inclusi. Una modifica ai sorgenti seguenti richiede nuova verifica.

| File | SHA-256 |
| --- | --- |
| `Cargo.toml` | `6737c29a7224b6ae06b91eb1e37fcd16392ae0461035ffbb188c69bb35726a6e` |
| `Cargo.lock` | `6f60e81a69616a62a2e8678e0e4d2b32b7e94315821800f7595107d8da17a58d` |
| `src/main.rs` | `b66b7f5d4ff566213a9b09709fbe4532174832aa9da8e5b298ad304411cebb34` |
| `src/notch_monitor.rs` | `01e13422203080dd70be0560d257036bdd82d815f284a821caefc8a86c1c7ab3` |
| `src/shell.rs` | `27f4686fbcbb4dd2d1fc761ae3d3a084dc0de36dc049e9a29e395a7bdbc1d4aa` |
| `src/pages/settings.rs` | `7e12aae8ce4399b4116cb0a5f237a672e2d87e327d1f826496d50194fa252b42` |
| `src/assist_layout.rs` | `8e5e74add72494d9ad4cf39d5b2f98f9867592fc5cc6d003d56e2c1c14ad8fd4` |
| `src/assist_overlay.rs` | `d4b255e78d2445b80455908b393d9480d9107f9d2c111835089e9ef5e20697eb` |
| `src/assist.rs` | `1c3ea634f4172b93146c8fb60c2ecd157bfc14b856d4f9ee74a6fac90f205d0c` |
| `src/call_session.rs` | `9f9fa204b4ec1b44449972a7975d8ed85094ece4b0f6b43eeb7881edf9902ee5` |
| `src/call_stt.rs` | `0329e03b88eefa43e15d5ca4c0994da517f2959a08fddde942a1b7cfc9f2e89b` |
| `src/call_ai.rs` | `ac9e49034d21eb209c10910065a3c865a362b6c05ca12ecc1ce00680c64e29ed` |
| `src/call_coach.rs` | `715a2596025ce4e216dd617993510b76b5addd5cd7ef5590567f6522afbe134f` |
| `src/codex_bridge.rs` | `4c0f3e8fae22a131d683fa85d1f59b11a99385359813ebe1dd55ad1d1ff6daae` |
| `src/codex_bridge/transport.rs` | `48962b5cd8e47b55252b21a89f6a033c311519aa9a381d7da175909b506cd93d` |
| `src/codex_server.rs` | `8417d1cc4112a511c6a6f26f566f56517a040e262c113f3aecb35fa06e9f5168` |
| `src/window_policy.rs` | `338f76cdbead84547c1bb4cfa252881b1eb769bc9aad22ce03895acfdc6ec645` |
| `tools/Setup-CallTranscription.ps1` | `0a1dde6d7ff6ffc9906694fa7225f363134fe7d09b3465d6d9bcc98d7a189397` |
