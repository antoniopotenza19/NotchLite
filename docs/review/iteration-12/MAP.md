# Mappa del cambiamento

Graphify AST locale: 49 file, 2128 nodi, 5337 archi, zero token LLM. Hash confrontati con i sorgenti compilati.

- `theme` fornisce token statici Graphite condivisi.
- `shell` disegna la navigazione e passa il bordo a `gfx`, che conserva una sola geometria corrente.
- `call_panel_size` legge preferenze limitate e conserva dimensioni in DIP.
- `assist_layout` calcola dimensioni entro l’area sotto la notch e la dashboard.
- `assist_overlay` mantiene BUTTON nativi, disegno GDI e sizing; salva solo alla fine del gesto o su azione esplicita.

Nessuna modifica ai motori ASR/AI. La mappa non verifica da sola input nativi, risorse o resa visiva.
