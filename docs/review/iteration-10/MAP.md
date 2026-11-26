# Mappa del cambiamento

Graphify locale AST, 46 file Rust, 1983 nodi e 4903 archi. Nessun LLM o installazione. Hash verificati contro SOURCE_SNAPSHOT.json dopo il congelamento dei sorgenti.

- `main` smista la diagnostica OS prima dell’avvio normale.
- `window_policy` interpreta due preferenze, decide gli stili e applica la richiesta alle finestre proprie. Lo stato per il disegno è memorizzato.
- `shell` applica la richiesta prima di mostrare il main, gestisce tray/recupero e revoca l’ascolto prima della distruzione.
- `settings` mostra lo stato attivo e modifica solo la chiave scelta conservando le altre preferenze fresche.
- `assist`, `assist_overlay` e `desktop_clock` applicano prima di mostrare e chiudono in caso di errore.

FFI, focus e backend di cattura richiedono verifiche runtime distinte: la mappa non le certifica.
