# Revisione indipendente di acquisizione e ponte locale

Data: 26 novembre 2026. Perimetro: una call confermata dall'utente, un'app
selezionata e un assistente che partecipa attivamente attraverso il ponte
locale. Revisione del sorgente con Ponytail Review del progetto e metodo
APTIOS per i confini di sicurezza. Non Ã¨ un audit completo del prodotto.

Il revisore di questo rapporto non Ã¨ autore dei tre moduli di acquisizione
o del ponte IPC. Ha implementato il modulo STT in un incarico separato:
**questo rapporto non presenta STT come revisione indipendente**. La sua
revisione da parte di un altro autore Ã¨ riportata in CONTROLLER-REVIEW.md.

## Copertura

| UnitÃ  | Copertura e confini seguiti |
| --- | --- |
| `src/call_audio.rs` | Letto integralmente: conferma, stato, generazioni, buffer, Stop e test. |
| `src/call_audio/windows.rs` | Letto integralmente: proprietario MTA, identitÃ  PID/FILETIME/sessione, callback, registrazioni e rilascio. |
| `src/call_audio/capture.rs` | Letto integralmente: attivazione asincrona, memoria COM, process loopback, formato e pacchetti PCM. |
| `src/call_coach.rs` | Letto integralmente: puntatore privato, segreto locale, framing, attese, suggerimenti, cancellazione e CLI. |
| Controller e UI | Ispezione mirata di `start_call`, `tick_call`, `stop_call`, azioni dei pulsanti e indicatore; non una verifica completa della UI. |
| STT, protocollo Codex, telefono, dipendenze binarie | Fuori dal perimetro indipendente di questo rapporto. |

## Rilievi corretti e riletti

### A9-CAP-1 â€” Stop e conferma potevano lasciare uno stato incoerente

Una notifica di errore sul worker poteva pubblicare generazione desiderata
zero prima di ottenere il mutex dello stato. Una conferma concorrente
poteva poi pubblicare la nuova generazione; Stop cancellava l'attivazione
pendente lasciando perÃ² una generazione desiderata diversa da zero. Il
worker rimaneva in `Stopping` fino a un altro Stop. Ãˆ un difetto di ciclo di
vita dimostrabile dal sorgente, non una prova di acquisizione non autorizzata.

Correzione riletta in `call_audio.rs:126`: Stop prende prima lo stesso mutex
usato dalla conferma, poi revoca la generazione e svuota lo stato. La
regressione `concurrent_stop_waits_for_confirm_state_before_revoking_generation`
copre l'interleaving con canali e mutex; crea soltanto un evento kernel,
senza COM o dispositivo audio.

### A9-IPC-1 â€” Un client incompleto poteva ritardare Stop continuando a inviare byte

Il vecchio `read_exact` usava un timeout di inattivitÃ  per ogni lettura.
Un peer locale non autenticato che continuava a inviare un byte prima del
timeout poteva tenere occupato il worker seriale; Stop ne attendeva la
terminazione. Un limite alla dimensione del messaggio non limitava il tempo
totale necessario a riceverlo.

Correzione riletta in `call_coach.rs:198`: lettura e scrittura hanno una
scadenza assoluta condivisa dall'intero pacchetto e controllano Stop tra le
operazioni. Il server ammette un secondo per il frame, con singole attese
di massimo 250 ms; le risposte long-poll hanno una scadenza separata. Non Ã¨
una promessa di deadline assoluta delle chiamate kernel.

La regressione aggiornata `stop_does_not_wait_for_an_incomplete_unauthenticated_packet`
mantiene un producer attivo per due secondi, con progressi ogni 20 ms, e
richiede che Stop torni entro un secondo. La precedente versione del test,
con peer semplicemente inattivo, non riproduceva il difetto ed Ã¨ stata corretta.
Il revisore non ha eseguito attacchi su servizi reali.

### A9-IPC-2 â€” Il puntatore locale non verificava tutti i componenti del percorso

Il primo codice verificava soltanto il file finale. Una home applicativa
reindirizzata o un padre junction potevano causare I/O prima di sapere che
il percorso fosse effettivamente locale. Non Ã¨ stato dimostrato un accesso
fra utenti o l'estrazione del segreto.

`validate_local_path` ora controlla drive locale, UNC e reparse point dal
padre verso il figlio, prima di creare o leggere il puntatore. La creazione
usa `CREATE_NEW` e una DACL protetta per proprietario e SYSTEM. Il controllo
di questa correzione Ã¨ statico; non Ã¨ stato preparato un share remoto.

## Confini confermati nel sorgente

- L'avvio del monitor legge solo metadati. La cattura richiede il candidato
  confermato, un'identitÃ  valida e una nuova generazione. `Starting` Ã¨
  visibile prima del ritorno della conferma; Stop resta disponibile anche
  durante avvio e arresto.
- La scelta Ignora è limitata all'occorrenza della comunicazione: quando
  termina, catalogo, controller e presentazione azzerano la soppressione.
  Una nuova call può quindi essere proposta anche con lo stesso PID e la
  stessa sessione WASAPI. Il riarmo cambia soltanto il suggerimento e la
  visibilità: non richiama la conferma, non crea una cattura e non avanza la
  generazione. Due regressioni coprono catalogo e presentazione senza
  inizializzare audio o GUI.
- Il device Ã¨ `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK`, con inclusione del
  processo selezionato e dei figli. Non esistono fallback a microfono o
  audio dell'intero sistema. Browser e alberi multiprocesso possono coprire
  piÃ¹ schede: il controller lo indica esplicitamente.
- Il processo resta posseduto tramite handle e FILETIME; la sessione viene
  ricontrollata dal worker. L'handle mantenuto evita il riuso del PID durante
  la vita dell'oggetto, coerentemente con la [spiegazione Microsoft](https://devblogs.microsoft.com/oldnewthing/20110107-00/?p=11803/).
- Parametri di attivazione in `VT_BLOB` sono allocati con `CoTaskMemAlloc`
  e posseduti dal `PROPVARIANT`. Un `Arc` li mantiene validi anche su
  completamento tardivo; due attivazioni pendenti sono il limite globale.
  Il callback segnala soltanto completamento e wake. Le interfacce audio
  vengono usate e rilasciate dal proprietario MTA. Il binding windows-implement
  0.58 espone `IAgileObject`, come richiesto dal [contratto Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nn-mmdeviceapi-iactivateaudiointerfacecompletionhandler).
- Ogni buffer WASAPI Ã¨ rilasciato con RAII; i campioni non allineati sono
  decodificati per byte. Il formato richiesto Ã¨ PCM16 mono 16 kHz. Sono ammessi
  al massimo un secondo per pacchetto, 32 chunk da 100 ms e un parziale;
  l'overrun arresta e scarta la coda. Stop revoca immediatamente i risultati
  prima di attendere il rilascio delle risorse.
- Il listener IPC usa soltanto IPv4 localhost con porta assegnata dal sistema.
  Il segreto casuale a 256 bit Ã¨ nel puntatore privato e non nelle risposte.
  Ogni richiesta verifica segreto, sessione e generazione. Framing massimo
  16 KiB; transcript 5 KiB, contesto 2 KiB, suggerimento 4 KiB. I controlli
  testuali e le dimensioni tengono limitato anche il JSON dopo escaping.
- Il protocollo ammette soltanto attesa del testo e pubblicazione di un
  suggerimento. Non espone esecuzione, approvazioni, lettura file arbitraria,
  host API o chiamate al modello. L'assistente deve essere attivo in una
  chat: il ponte non costituisce un servizio AI autonomo o sempre collegato.
- Nel controller, un errore fatale STT ritorna al chiamante: `tick_call`
  richiama Stop e revoca l'audio prima di attendere la pulizia IPC/STT.
  Stop conserva il pannello e il testo già visibile; Close richiede la
  chiusura della presentazione. È una rilettura del percorso, non una prova
  di rendering o di interazione Windows.
- I cursori emessi sono limitati a 32, scadono dopo 60 secondi e sono monouso.
  Risposte vecchie o di un'altra sessione vengono rifiutate. Stop revoca la
  sessione, svuota lo stato condiviso e rimuove il puntatore posseduto.

## Evidenze e limiti

Il revisore ha svolto lettura e confutazione dei casi sul sorgente, senza
scanner, accesso a credenziali, cattura audio, GUI o prove su endpoint di
produzione. I test e le prove headless eseguiti dal coordinatore sono
registrati separatamente in VALIDATION.json; non sono esecuzioni del revisore.
I riepiloghi finali letti riportano 131 test passati, zero errori, e build
release GNU riuscita in 29,28 secondi. Le regressioni di concorrenza e del
peer incompleto sono incluse in quella suite; non attivano una cattura.
La build successiva al riarmo delle nuove call ha SHA-256
`96cb6d39c1c083c8db41f609efd58bc9b2d2267903e5714e06c2db49d39d5598`.

Sullo stesso binario finale con SHA-256
`96cb6d39c1c083c8db41f609efd58bc9b2d2267903e5714e06c2db49d39d5598`,
il coordinatore ha inoltre completato queste prove headless:

- `--call-transcription-check` su sei secondi del WAV pubblico in inglese:
  risultato non vuoto in 281 ms. È una prova del file e del worker locale,
  distinta dall'acquisizione Windows e dall'accuratezza sulle call reali.
- `--call-coach-check` con il testo della fixture, lettura con `--call-await`
  da questa chat attiva e pubblicazione di una risposta con `--call-suggest`:
  risposta accettata e `suggestionReceived: true` sul produttore.
- Pulizia del puntatore privato verificata dopo Stop. Le prove hanno
  riportato `audioCaptured: false` e `guiOpened: false`.

Questi esiti sono attribuiti al coordinatore e non sono test eseguiti dal
revisore. Non vengono pubblicati transcript, risposta, token o identificatori
di sessione. La prova del ponte riguarda un assistente attivo in questa chat;
non dimostra un collegamento AI autonomo quando la chat non sta lavorando.

Restano da verificare cattura WASAPI reale dopo consenso, attivazione tardiva
e guasti dei device sul sistema reale, UI/focus/DPI/hotplug e una call completa.
Il gate build 20348 segue il campione Microsoft; non certifica ogni variante
di Windows. Il costo RAM completo e l'accuratezza STT non sono dedotti da
questa revisione.

Verdetto sul sorgente ispezionato: i rilievi sono corretti e non rimangono
blocchi aperti individuati nel perimetro. L'evidenza funzionale della preview
rimane circoscritta ai test del coordinatore e ai limiti indicati sopra.
Questo rapporto non certifica l'intero prodotto.

## Snapshot dei file esaminati

| File | SHA-256 |
| --- | --- |
| src/call_audio.rs | 3744f2f0c72026f5f06c135b77eb9ca07b9ab598be8696ace4d7f000934168eb |
| src/call_audio/windows.rs | 33f41806b9d373879f49a2e9ce46fb0a16669a0eb74b4b3f0755a9a9ac70eb03 |
| src/call_audio/capture.rs | 08cd5074597231ca744871c1dd0f00da81ab2f4ed556b4c8ba34dbccecf6cc8f |
| src/call_coach.rs | 1e0c9fdc909c3ef4c930211bf9969cd34f14c99fc3071f5d4dbda459b771ce65 |
| src/assist.rs | 272998087e79c289ed9a34e2da005d66b7542d7ec01ab614e6b648f76b84d24f |
| src/assist_overlay.rs | aa6fb10fc961159f858ec38693b27d976905324f8265767eefb13d8e058f29b7 |
