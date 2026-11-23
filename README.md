# NotchLite

NotchLite è una versione derivata di [Ada Notch](https://github.com/Yemresalcan/ada-notch), con un profilo essenziale e correzioni di affidabilità e validazione. Gran parte del codice resta quello originale: non è una riscrittura integrale. Nome e identità grafica sono provvisori.

Obiettivo: realizzare gradualmente una notch Windows semplice e leggera, prendendo le funzioni pubblicamente descritte da NotchPixel come riferimento. [Prova la preview portatile](https://github.com/antoniopotenza19/NotchLite/releases) oppure leggi [le istruzioni](docs/TRY-IT.md).

## Stato reale

È disponibile una build portatile sperimentale per Windows x64 GNU, compilata in release con Rust 1.100.0. La versione 0.1.6 supera 59 test Rust, inclusi scambi TLS su loopback: vedi [validazione](docs/review/iteration-6/VALIDATION.json). Il pacchetto portatile contiene notchlite.exe, istruzioni e licenze. Anche APK Android e build iOS Simulator unsigned compilano: tre test parser per piattaforma, [esiti CI](docs/review/iteration-6/CI.json). [Esito della prima revisione](docs/review/SUMMARY.md).

Il doppio clic apre il pannello, visibile anche in taskbar/Alt-Tab; rilanciare l'app riapre l'istanza esistente. Escape torna alla capsula. L'avvio automatico usa --background. Apertura e navigazione sono state osservate nella 0.1.2. La 0.1.6 è stata verificata soltanto da riga di comando: nessuna apertura dell'app o modifica al desktop. Resa dei preset e dell'orologio, navigazione, incolla, uscita dal menu, DPI e flussi completi restano da collaudare.

**Aspetto della notch:** quattro preset in Settings > Appearance: Original, Graphite, Midnight e Rose. La nuova barra a icone rende direttamente selezionabili le pagine abilitate; nome della sezione leggibile sotto il contenuto e Ctrl+Tab/Ctrl+Shift+Tab fuori dagli editor. Il riferimento visivo è Noci Share, con implementazione e risorse del nostro fork. [Navigazione e schema offline](docs/MODERN-NAVIGATION.md), [palette](docs/APPEARANCE.md), [revisione](docs/review/iteration-5/REVIEW.md).

**Telefono (preview 0.1.6):** [QR e scambio file sulla LAN](docs/PHONE-SHARE.md) con [companion Android e iPhone](docs/COMPANION.md), certificato verificato dal QR e consenso sul PC. Settings > Phone link (LAN) è spento anche nei profili esistenti; il server parte soltanto con Start QR pairing. Sessioni temporanee, un file per direzione, massimo 256 MiB. Nessun tunnel o cloud. Bluetooth è un incremento successivo, non ancora attivabile. Restano da provare GUI e telefoni reali; su iPhone serve una build firmata.

**Orologio indipendente:** la 0.1.4 aggiunge Settings > Desktop clock, disattivato di default. Usa una finestra GDI, il timer già presente e i colori del preset. Ora 24 ore e data locale, posizione fissa sul monitor principale; si mostra solo quando il desktop è attivo e lascia passare i clic. Non è ancora un widget ancorato dietro le icone. [Comportamento e limiti](docs/DESKTOP-CLOCK.md), [revisione del modulo](docs/review/iteration-4/REVIEW.md).

La [misura RAM della 0.1.2](docs/PERFORMANCE-0.1.2.md) resta storica: 43,6 MiB residenti medi in background e 42,8 MiB con il pannello aperto, nel profilo base inattivo, memoria GPU esclusa. La 0.1.6, il collegamento telefono e l'orologio non sono stati misurati.

**Obiettivo visivo:** personalizzare anche il desktop nel suo insieme. Il primo orologio è un modulo sperimentale da collaudare; barra/dock, sfondo coordinato, vetro e altri widget sono ancora da implementare. Vedi [esperienza desktop completa](docs/DESKTOP-EXPERIENCE.md).

Le funzioni ereditate sono codice presente nella base, non funzionalità certificate sul tuo PC. Vedi [ROADMAP.md](docs/ROADMAP.md) per le parti mancanti.

## Modifiche iniziali

- Eseguibile previsto: `notchlite.exe`; identità della finestra, mutex, voce di avvio e cartelle dati separati da Ada.
- Primo avvio: notch minimale, preset Graphite, senza mascotte; attivi soltanto deposito file, lista attività e timer focus. Le configurazioni esistenti sono conservate.
- Gli altri moduli già presenti restano attivabili in Settings. I moduli disattivati non vengono costruiti all'avvio.
- Il profilo iniziale viene scritto solo se manca il file delle impostazioni. Le scelte successive vengono conservate.
- Il timer da 500 ms della mascotte viene attivato solo quando la mascotte è abilitata.
- Intervallo richiesto delle transizioni portato da 1 a 16 ms; non è una misura della frequenza effettiva.
- Restano il controllo generale ogni secondo e, quando si abilita la musica, i worker e le animazioni media ereditati. Il programma non promette CPU zero.
- Logo e risorse grafiche provengono ancora da Ada e dai componenti indicati in NOTICE; la nuova identità grafica è da realizzare.

Nella 0.1.2: scrittura iniziale del profilo senza file finali parziali o sovrascritture, sospensione del rendering quando la capsula è nascosta dal fullscreen, apertura esplicita e incolla testuale limitato nei campi.

## Team di sviluppo

[AGENTS.md](AGENTS.md) collega il [team](team/DEVELOPMENT.md), le skill locali Ponytail e lo script [Update-CodeGraph.ps1](tools/Update-CodeGraph.ps1). Graphify genera una mappa AST locale dei sorgenti; Ponytail guida implementazione e review. Sono strumenti di sviluppo, senza nuove dipendenze nel processo dell'app.

## Compilazione

Per usare il pacchetto portatile non serve installare Rust. Per ricompilare: Windows x64, Rust e compilatore C per ring; toolchain MSVC con Windows SDK oppure GNU con MinGW completo. Le toolchain della sessione restano nella cartella di lavoro. Non è richiesto un browser incorporato per la UI. Le licenze delle dipendenze Rust sono in [licenses/rust](licenses/rust/README.md).

```powershell
cargo test --locked
cargo build --release --locked
.\target\release\notchlite.exe
```

Il comando finale avvia manualmente l'app. Nessun avvio automatico è stato impostato durante la preparazione.

Dati del fork:
- `%APPDATA%\NotchLite\settings.txt`: impostazioni.
- `%APPDATA%\NotchLite`: liste e calendario.
- `%LOCALAPPDATA%\NotchLite`: cache e deposito file.

Il doppio clic apre il pannello e lo mantiene visibile; Escape torna alla capsula (durante una modifica, prima chiude l'editor). In modalità capsula, passare il mouse per aprirla e allontanarlo per richiuderla. Settings è l'ultima pagina. Clic destro sulla notch o sull'icona nell'area di notifica per aprire il menu con Exit.

Abilitando musica, meteo o calcio si abilitano anche le richieste di rete del codice ereditato: rispettivamente testi delle canzoni, previsioni/geolocalizzazione e risultati sportivi. Il profilo iniziale li lascia spenti. L'opzione avvio con Windows resta una scelta manuale.

## Misurazione

Aprire l'app compilata, lasciarla stabilizzare e trovare l'ID del processo:

```powershell
Get-Process notchlite | Select-Object Id
.\tools\Measure-Notch.ps1 -TargetProcessId 1234 -Label idle -OutputDirectory .\measurements
```

Sostituire 1234 con l'ID effettivo. Lo script osserva soltanto il processo indicato; non lo avvia e non lo chiude. Genera CSV e JSON con memoria privata, working set, CPU normalizzata, thread e handle. Non misura direttamente memoria GPU né latenza visiva.

Vedi [PERFORMANCE.md](docs/PERFORMANCE.md) per obiettivi e scenari.

## Provenienza e licenza

Base: Ada, revisione `5a1e17f040672a71685872b1b234e01d02a47bb5`.
Licenza mantenuta: Apache-2.0. Sono conservati LICENSE, NOTICE e README originale in docs/UPSTREAM-README.md. I file modificati riportano un avviso; il diff iniziale è in docs/initial-changes.patch.

Il progetto conserva la provenienza e la storia Git di Ada. Non include codice o asset di NotchPixel.
