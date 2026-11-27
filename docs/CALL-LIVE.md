# Suggerimenti AI automatici e ponte locale opzionale

La trascrizione locale funziona anche senza AI. Quando l'utente abilita i
suggerimenti nella sessione di ascolto confermata, `Coach::start` avvia un
worker che usa il CLI ufficiale e il server Codex condiviso con accesso
ChatGPT. La chat desktop non deve restare in esecuzione attiva. NotchLite
non legge credenziali e non usa un'API key come ripiego.

## Connessione e limiti

Il worker controlla o avvia il daemon canonico mediante il comando ufficiale
già usato dal pannello Codex. Si collega con un proprio processo proxy e non
modifica le chat esistenti. Questo percorso è vincolato al server verificato
`0.171.0-alpha.13.2`; versioni o metodi non compatibili producono un errore
visibile, senza inviare la trascrizione a un normale turno con strumenti.

Per ogni sessione crea una chat tecnica persistente contenente esclusivamente
istruzioni statiche e un turno di preparazione `PRONTO`. La trascrizione e
il contesto della call vengono inviati solo a `thread/sample`: il server
crea un fork temporaneo, intercetta una sola risposta prima che Core possa
eseguirne gli strumenti e rimuove il fork. Le risposte non testuali vengono
scartate. Il campione richiede `disableMcp`; le chiamate a strumenti vengono
comunque intercettate e scartate prima del dispatch Core. Ambiente, hook e
ricerca web sono disabilitati nella chat tecnica. Il contenuto della call è trattato come
dato non fidato anche nel prompt. Non viene inoltrato a `turn/start`.

Una sola richiesta può essere attiva; gli aggiornamenti vengono accorpati
al testo più recente e partono a distanza di almeno 10 secondi. Il campione
usa fino a 2500 byte recenti della trascrizione, fino a 800 byte di contesto
e un prompt inferiore a 4000 byte. I suggerimenti hanno al massimo 2048 byte.
La risposta non è una trascrizione completa e può essere errata.

Il pannello mostra preparazione, attesa del testo, generazione ed eventuali
errori. Espone il numero di richieste, includendo il turno iniziale, i token
noti e la quota dell'account solo quando il server li fornisce. I token
misurati possono essere parziali; la percentuale è dell'account, non una
misura esclusiva della call. Anche queste richieste consumano i limiti
dell'abbonamento. Un errore AI non interrompe la trascrizione locale.

`Stop` revoca gli aggiornamenti, cancella contesto e coda locali e richiede
la cancellazione del proprio campione, l'interruzione del proprio eventuale
turno tecnico e l'archiviazione della propria chat tecnica. Non arresta il
daemon condiviso. La UI attende al massimo circa 900 ms il worker AI; il
completamento della pulizia può continuare in background con timeout. La
chat tecnica archiviata non è una cancellazione permanente, ma non contiene
testo della call. Un servizio online riceve comunque il testo inviato ai
campioni e valgono le condizioni dell'account utilizzato.

## Verifica AI senza audio o finestre

```powershell
.\notchlite.exe --call-ai-check
```

Il comando usa solo frasi sintetiche predefinite e le stesse policy della
produzione. Richiede una risposta testuale, poi avvia un secondo campione
e richiede Stop appena l'invio risulta effettuato. Il riepilogo riporta la
prima risposta, richieste, metriche disponibili, durata di Stop, svuotamento
dei risultati locali e l'eventuale conferma del server alla cancellazione.
Una conferma non prova che il servizio remoto abbia già interrotto ogni
elaborazione. Non acquisisce audio, non apre GUI e usa i limiti
dell'abbonamento. Lo scarto delle risposte non testuali ha un test unitario;
questa prova runtime non forza il modello a chiamare strumenti e non viene
descritta come test dinamico di mancata esecuzione.

## Ponte manuale opzionale

Il listener locale resta disponibile per un assistente che sta già lavorando
in una chat attiva. Questo percorso manuale richiede una lettura esplicita
della chat; il worker automatico descritto sopra è indipendente.

Il listener usa esclusivamente `127.0.0.1`, una porta temporanea e un token
casuale di 256 bit. Il protocollo binario `NLC1` non è HTTP. Il file
`%LOCALAPPDATA%/NotchLite/call-coach.json` contiene solo indirizzo, token,
identità e generazione, con ACL protetta per proprietario e SYSTEM.
Trascrizione e suggerimenti del ponte restano in memoria. Stop chiude il
listener e rimuove il proprio file di individuazione.

```powershell
.\notchlite.exe --call-await --cursor 0 --timeout-seconds 20
# Il JSON deve esistere già, in UTF-8 senza BOM, con percorso assoluto locale.
$replyFile = (Resolve-Path -LiteralPath '.\risposta.json').Path
.\notchlite.exe --call-suggest --input-file $replyFile
```

`--call-await` restituisce il testo successivo al cursore o attende al massimo
25 secondi. Include `session_id`, `generation`, `sequence`, `context` e
`transcript`, esplicitamente marcati come dati non fidati. Il JSON di risposta
contiene `session_id`, `generation`, `sequence` e `text`; è accettato anche
su standard input. Non interpolare il testo in comandi di shell.

Il ponte accetta solo suggerimenti testuali e non interpreta strumenti o
comandi. Il cursore manuale deve appartenere alla sessione corrente, essere
stato emesso da meno di 60 secondi e non essere già stato risposto. Le
risposte automatiche del worker posseduto hanno un controllo separato sulla
sequenza, così una preparazione lenta non le fa scadere. La finestra manuale
contiene al massimo 5000 byte recenti, il contesto 2048 e la risposta 4096.

```powershell
.\notchlite.exe --call-coach-check
$testFile = (Resolve-Path -LiteralPath '.\trascrizione-di-prova.txt').Path
.\notchlite.exe --call-coach-check --text-file $testFile
```

Questa seconda verifica apre esclusivamente il ponte manuale per al massimo
60 secondi, con una frase sintetica o un file di prova scelto esplicitamente.
Attende una risposta da un secondo client e non invoca modelli né audio.
I test unitari usano file temporanei separati e dati sintetici.

## Contratto verificato alla fonte

Il percorso dipende dall'API sperimentale del CLI verificato, non da un'API
stabile per applicazioni terze. Il comportamento di intercettazione e
pulizia è stato letto nella
[fonte ufficiale `thread_sample.rs`](https://github.com/openai/codex/blob/ad9d659a8491fc5169a00dcf7b7e6ca8a87adf69/codex-rs/app-server/src/thread_sample.rs).
Una modifica di quel contratto richiede nuova verifica prima di ampliare la
compatibilità.
