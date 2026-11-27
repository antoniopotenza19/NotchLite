# Mappa del cambiamento

Graphify AST locale: 48 file Rust, 2091 nodi, 5229 archi. Snapshot verificata sui sorgenti compilati. Nessun LLM nella mappatura.

- `notch_monitor` e Settings scelgono il dispositivo salvato; `shell` sposta il main prima del sync assistente.
- `assist_layout` riserva la fascia del main e calcola controlli; `assist_overlay` usa i bounds reali e segue la coda solo se già in fondo.
- `call_session` richiede nuova voce dopo il taglio; `call_stt` usa decoding e VAD locali.
- `call_coach` collega i segmenti al worker `call_ai`, che usa il trasporto ufficiale condiviso e campionamenti testuali con revoca.

Il grafo può omettere FFI, callback e comportamento runtime; non certifica resa visiva, comprensione di una call o sicurezza generale.
