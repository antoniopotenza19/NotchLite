# Provare NotchLite

La preview 0.1.8 include **Settings > Codex dashboard** e **Call assistant**, entrambi spenti inizialmente. I pannelli sono ancorati al secondario, rispettivamente in alto a sinistra e a destra. Con più secondari usa **Assistant monitor**; senza un secondario valido non ripiegano sul primario.

Il bridge si collega a un daemon Codex locale condiviso, usando l'accesso ChatGPT del CLI ufficiale. Sul PC di sviluppo il server è già stato preparato. Nel pannello puoi inserire la cartella del progetto, premere **Nuova chat** e poi **Send**; le nuove chat partono in sola lettura. **Avvia server locale** serve quando il pacchetto ufficiale completo è installato ma il server è spento. Per un altro PC: [preparazione del server](CODEX-SHARED-SERVER.md).

Le chat già aperte nell'attuale Codex Desktop incorporato **restano separate**. Token e limiti vengono dal server, RAM/CPU sono totali dei processi osservati, non costi per chat. Domande ricevute hanno risposte esplicite; autorizzazioni richiedono un client collegato allo stesso server. [Dettagli e limiti](CODEX-DASHBOARD.md).

Per le call prepara il motore con `tools/Setup-CallTranscription.ps1` (già fatto sul PC di sviluppo), abilita **Call assistant**, verifica l'app e scegli **Trascrivi** oppure **Trascrivi + AI**. L'audio resta locale; nella seconda modalità segui [CALL-LIVE.md](CALL-LIVE.md) dalla chat AI da usare. Stop ferma l'ascolto e conserva il testo visibile. Il rilevamento non avvia nulla automaticamente. [Uso e limiti](CALL-ASSISTANT.md).

Diagnostica senza finestre: `notchlite.exe --codex-check` verifica solo i metadati locali; `notchlite.exe --codex-connect-check` apre il bridge reale e legge account/quote, senza inviare messaggi. `--codex-server-start` richiede esplicitamente l'avvio persistente del daemon. Nessuno di questi comandi apre l'app grafica o modifica le preferenze NotchLite.

1. Scarica NotchLite-portatile.zip dalla sezione Releases del repository.
2. Estrai l'intera cartella.
3. Chiudi una precedente NotchLite eventualmente aperta, poi apri il nuovo notchlite.exe con un doppio clic. Non serve installare Rust.
4. Il pannello si apre in alto al centro del monitor principale ed è presente in taskbar/Alt-Tab. La barra a icone cambia pagina; il nome della sezione compare sotto il contenuto. Ctrl+Tab/Ctrl+Shift+Tab cambiano pagina fuori dagli editor. Se la barra non entra, restano frecce e puntini.
5. Escape riduce il pannello alla capsula. Se stai scrivendo, il primo Escape annulla l'editor. La capsula si apre al passaggio del mouse; rilanciare l'EXE o usare Open panel dal menu la apre in modo persistente.

Il profilo iniziale attiva deposito file, attività e timer focus. Settings è l'ultima pagina e permette di abilitare gli altri moduli già presenti. Le funzioni avanzate della roadmap restano da realizzare. L'interfaccia ereditata è attualmente in inglese.

Nella 0.1.5 la prima riga di Settings, **Appearance**, passa fra Original, Graphite, Midnight e Rose e salva la scelta. Un profilo nuovo parte da Graphite; un profilo esistente senza questa voce conserva Original. Il cambio riguarda soltanto NotchLite: non applica temi o sfondi di Windows. Vedi [APPEARANCE.md](APPEARANCE.md).

La 0.1.4 aggiunge **Desktop clock**, spento di default. Quando vorrai collaudarlo, attivalo in Settings e torna al desktop per vederlo: la prima versione si nasconde quando un'altra app è attiva. Lo stesso interruttore lo disattiva. Posizione fissa sul monitor principale, ora 24 ore e data locale. È un modulo sperimentale non ancora avviato nella GUI; vedi [DESKTOP-CLOCK.md](DESKTOP-CLOCK.md).

Questa build è preparata con test e compilazione da riga di comando. La GUI dei preset e del widget non viene aperta durante la preparazione, per richiesta dell'utente. Resa e interazione richiedono ancora verifica.

La preview 0.1.6 aggiunge **Phone link (LAN)**, inizialmente spento: abilitalo in Settings, apri Telefono, scegli la rete e premi **Start QR pairing**. Usa la [companion Android/iPhone](COMPANION.md) per leggere il QR, confronta i codici e approva sul PC. Per ricevere un file devi accettarlo sul PC; per inviarlo seleziona Telefono e trascina un singolo file. Massimo 256 MiB per file. **Disconnect** termina la sessione. [Protocollo e limiti](PHONE-SHARE.md). Il QR non installa la companion iPhone, che richiede firma Apple. Il Bluetooth non è implementato.

Per uscire, fai clic destro sulla notch o sull'icona nell'area di notifica, quindi scegli **Exit**. Se l'icona è nascosta, apri le icone nascoste accanto all'orologio. Il menu sulla notch è nuovo nella 0.1.2 e resta da collaudare.

La modalità --background avvia soltanto la capsula, che si nasconde quando Windows segnala fullscreen/presentazione. Il pannello aperto esplicitamente resta visibile finché viene ridotto; la sospensione grafica riprende il normale comportamento della capsula.

I dati utente restano in %APPDATA%/NotchLite e %LOCALAPPDATA%/NotchLite; il pacchetto non abilita automaticamente l'avvio con Windows. La cartella dell'eseguibile è portabile, i dati seguono queste cartelle Windows.
