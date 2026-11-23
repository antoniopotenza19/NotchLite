# RAM e CPU — NotchLite 0.1.2

Misura del 23 novembre 2026 sul PC della sessione, build release Rust 1.100.0 x64 GNU. Due avvii separati, 15 secondi di stabilizzazione e 60 campioni a intervalli di circa un secondo per scenario. Durata osservata circa 60,6 secondi per ciascuno.

| Scenario | Working set medio MiB | Picco campionato MiB | Memoria privata media MiB | Picco privato MiB | CPU media % totale PC |
|---|---:|---:|---:|---:|---:|
| Capsula / background, inattiva | 43.6 | 43.7 | 45.4 | 45.4 | 0.0000 |
| Pannello aperto, inattivo | 42.8 | 46.4 | 40.3 | 45.7 | 0.0258 |

Profilo verificato dopo il campione: Shelf, Todo, Focus e Settings; mascotte spenta. Musica, rete/meteo, monitor di sistema e notifiche disabilitati. Nessun file aggiunto e nessun timer avviato. Nel caso background la visibilità segue la politica fullscreen; non è stato imposto lo stato del desktop o di altre app.

Il working set è la RAM residente del processo, incluse pagine condivise. La memoria privata è una vista diversa della memoria impegnata dal processo: non va sommata al working set. I picchi sono massimi campionati, non una garanzia sul massimo fra i campioni.

Lo 0% della CPU in background significa nessun incremento rilevato dal contatore in questo intervallo e alla sua risoluzione, non consumo nullo per sempre. La misura non include memoria GPU, DWM, altri processi, compilatore o Codex. Non è un benchmark con tutte le funzioni attive né una prova di uso interattivo o lunga durata.

Il budget iniziale di 40 MiB di memoria privata non è ancora raggiunto in tutti gli scenari. I due avvii hanno distribuzioni diverse di cache e pagine residenti; non si può dedurre che aprire il pannello riduca strutturalmente la memoria. Il campione storico della 0.1.1 non è un confronto controllato prima/dopo.

Fonti grezze sintetizzate: [background](benchmarks/v0.1.2-background.json), [pannello](benchmarks/v0.1.2-panel-open.json). CSV e identificativi del processo restano nelle evidenze locali.

## Aspetto completo di Windows

La misura riguarda la notch attuale. Barra/dock, sfondo coordinato, orologio e widget desktop non sono ancora implementati. [DESKTOP-EXPERIENCE.md](DESKTOP-EXPERIENCE.md) definisce ora la priorità visiva complessiva, con profili Leggero e Vetro e budget separati. Questi componenti futuri non possono essere attribuiti ai numeri qui misurati.
