# NotchLite

NotchLite è una versione derivata di [Ada Notch](https://github.com/Yemresalcan/ada-notch), con un profilo essenziale e correzioni di affidabilità e validazione. Gran parte del codice resta quello originale: non è una riscrittura integrale. Nome e identità grafica sono provvisori.

Obiettivo: realizzare gradualmente una notch Windows semplice e leggera, prendendo le funzioni pubblicamente descritte da NotchPixel come riferimento. [Prova la preview portatile](https://github.com/antoniopotenza19/NotchLite/releases) oppure leggi [le istruzioni](docs/TRY-IT.md).

## Stato reale

È disponibile una build portatile sperimentale, compilata in release con Rust 1.100.0 per Windows x64 GNU. Dopo la revisione, la suite normale passa 15 test su 15. Il pacchetto portatile separato contiene notchlite.exe, istruzioni e licenze. [Esito della revisione](docs/review/SUMMARY.md).

La finestra overlay non è selezionabile dal controllo automatico dell'interfaccia usato nella sessione: hover, navigazione, trascinamento e resa visiva restano da verificare manualmente. Vedi docs/BUILD-VALIDATION.md per le verifiche svolte e i loro limiti.

Le funzioni ereditate sono codice presente nella base, non funzionalità certificate sul tuo PC. Vedi [ROADMAP.md](docs/ROADMAP.md) per le parti mancanti.

## Modifiche iniziali

- Eseguibile previsto: `notchlite.exe`; identità della finestra, mutex, voce di avvio e cartelle dati separati da Ada.
- Primo avvio: notch minimale, senza mascotte; attivi soltanto deposito file, lista attività e timer focus.
- Gli altri moduli già presenti restano attivabili in Settings. I moduli disattivati non vengono costruiti all'avvio.
- Il profilo iniziale viene scritto solo se manca il file delle impostazioni. Le scelte successive vengono conservate.
- Il timer da 500 ms della mascotte viene attivato solo quando la mascotte è abilitata.
- Intervallo richiesto delle transizioni portato da 1 a 16 ms; non è una misura della frequenza effettiva.
- Restano il controllo generale ogni secondo e, quando si abilita la musica, i worker e le animazioni media ereditati. Il programma non promette CPU zero.
- Logo e risorse grafiche provengono ancora da Ada e dai componenti indicati in NOTICE; la nuova identità grafica è da realizzare.

## Compilazione

Per usare il pacchetto portatile non serve installare Rust. Per ricompilare: Windows x64 e toolchain Rust MSVC con linker MSVC/Windows SDK, oppure Rust GNU con il linker MinGW necessario. La build preparata nella sessione usa gli installer standalone ufficiali Rust GNU nella cartella di lavoro, con hash verificati. Non è richiesto un browser incorporato per la UI.

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

Passare il mouse sulla notch per aprirla; allontanarlo per richiuderla. Settings è l'ultima pagina. Uscita dal menu dell'icona nell'area di notifica.

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
