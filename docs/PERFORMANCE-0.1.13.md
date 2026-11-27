# Risorse e metadati NotchLite 0.1.13

27 novembre 2026. Binario GNU installato, SHA-256
`894ad31219db0fcf20dec7654b12b18036e2c1a87f39ee749fa342843aa1f6b8`. Preferenze conservate.

Campione di 22.926 secondi, sola UI con notch e reader call
visibili, ascolto spento: working set massimo **77.47 MiB**,
commit privato massimo **90.90 MiB**, CPU media
normalizzata su 8 processori logici **0.23%**.
Fino a 21 thread e 1357 handle osservati.
Il codice non aggiunge worker o timer; DWM e librerie di sistema possono
creare proprie risorse. Non è una comparazione controllata con la 0.1.12.

La misura esclude Whisper, server Codex, memoria GPU e compositore DWM.
La trascrizione attiva costa separatamente: [benchmark precedente](review/iteration-13/TRANSCRIPTION.md),
profilo invariato. La verifica file-only corrente ha confermato ancora la GPU
su un WAV pubblico di sei secondi in 5262 ms: singola prova, non benchmark
live o limite di latenza. Il tempo varia con cache e carico.

Lettura di soli metadati delle finestre possedute: due schede visibili,
controlli di configurazione nascosti, Stop e toggle presenti, ascolto spento,
nessuna intersezione con la notch. DWM backdrop letto come 3 (transient),
HRESULT 0. Questo conferma la richiesta accettata; non una prova visiva
di blur, testo o tastiera. Nessun desktop/input/screenshot usato.

Il controllo audio ha letto un render attivo e zero sessioni Teams in quel
momento: non è un collaudo della call segnalata. Nessuna cattura o microfono.
[Validazione e limiti](review/iteration-14/VALIDATION.json).
