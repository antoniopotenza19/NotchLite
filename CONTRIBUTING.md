# Contribuire a NotchLite

NotchLite è una versione derivata di Ada, in fase sperimentale. Mantieni attribuzioni e licenze nei contributi.

Prima di cambiare una funzione, leggi i chiamanti e individua un caso concreto. Preferisci la correzione minima, con un test che fallirebbe se il difetto ricomparisse. Non aggiungere librerie per funzionalità già disponibili in Rust o nelle API Windows.

Esegui `cargo test --locked` e `cargo build --release --locked`. Per modifiche alla UI prova avvio/uscita, tastiera, hover e DPI interessati. Distingui prove svolte da controlli statici e limita le affermazioni sulle prestazioni ai campioni effettivi.

I limiti noti sono raccolti in docs/review/quality.md, docs/review/security/REPORT.md e docs/ROADMAP.md.
