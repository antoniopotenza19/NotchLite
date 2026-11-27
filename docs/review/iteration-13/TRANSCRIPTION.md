# Confronto italiano locale — 27 novembre 2026

Scelta: Whisper Large-v3 Turbo Q5_0, runtime Vulkan distribuito da Subtitle
Edit `whispercpp-195` (CLI 1.9.5-dev), device 0 RX 9070 XT, quattro thread,
Flash Attention esplicitamente disabilitata. CPU Small Q5_1 resta una scelta
esplicita alternativa. I pin e le licenze sono in [CALL-TRANSCRIPTION](../../CALL-TRANSCRIPTION.md).

## Metodo e risultati

Stesse 12 frasi complete di italiano parlato, 82,8 secondi, 159 parole di
riferimento: prime 12 righe test con durata fra 4 e 8 secondi, in ordine,
scelte prima di vedere gli output. Fonte: [Google FLEURS](https://huggingface.co/datasets/google/fleurs),
CC-BY-4.0, credito Google/FLEURS authors, tramite [mirror mteb](https://huggingface.co/datasets/mteb/fleurs)
revision `c170aa8edef91f66587fc80354e143dfcc7aaaa6`, `it_it`, `test`.
Indici: 4, 11, 25, 74, 84, 139, 154, 178, 191, 193, 195, 221.
Non vengono distribuiti audio, trascrizioni o URL firmati del dataset.

WER ortografico: NFKC, minuscole, separazione punteggiatura/apostrofi,
distanza di Levenshtein sulle parole; numeri e accenti non corretti a mano.
Lingua `it`, beam/best-of 5, contesto completo, Silero VAD 100/300/160 ms.
Ogni frase avvia un processo nuovo, come il worker del prodotto.

| Profilo | Modifiche/parole | WER | Mediana processo |
| --- | ---: | ---: | ---: |
| Base Q5_1 CPU 1.9.2, 2 thread | 39/159 | 24,53% | 1,672 s |
| Small Q5_1 CPU 1.9.2, 4 thread | 27/159 | 16,98% | 3,109 s |
| Turbo Q5_0 Vulkan 195, no flash, 4 thread | 6/159 | 3,77% | 1,134 s |

La prova Turbo finale è a cache calda: intervallo 1,073–1,165 s. La prima
prova preliminare 195 aveva richiesto 3,658 s. Small ha avuto un outlier
di 13,303 s. Non sono massimi garantiti né latenze audio→pannello.
Versione motore e backend differiscono: il confronto non attribuisce alla
GPU un aumento dell'accuratezza del modello.

Tre frasi trasformate in Opus 24 kbps e attenuate di 18 dB: Turbo 0/41
modifiche contro Base 8/41 su file interi. Riproducendo anche Segmenter e
merge Rust reali: Turbo 0/41 sia sui tre originali sia sui degradati;
Base rispettivamente 8/41 e 7/41. Nessuna modifica arbitraria alla soglia
di volume è giustificata da questo piccolo campione. I nove segmenti Rust
sono stati decodificati senza aprire endpoint audio.

Risposte sintetiche “sì”/“no” corrette, silenzio/click senza parole.
Complessivamente 29 processi GPU dopo autorizzazione esplicita: tutti exit 0,
senza retry automatici. Il candidato precedente Vulkan 1.9.2 aveva avuto
un crash isolato 0xC0000409 sul degradato, non riprodotto nelle sei riprove.
Non è stata dimostrata la causa: per l'integrazione si usa il runtime 195
con no-flash e il controllo dell'offload, senza ripiego CPU nascosto.

## Risorse

Su Ryzen 7 7800X3D / RX 9070 XT, campionamento processo circa ogni 10 ms:
picchi osservati nelle 28 decodifiche 195, **129,95 MiB working set** e
**1726,13 MiB commit privato**. La precedente inizializzazione preliminare
aveva raggiunto 199,90 MiB e 1790,36 MiB rispettivamente. Non sono un budget
massimo o misure della RAM complessiva dell'app.

Contatori Windows PDH del PID in una prova separata di sei secondi:
**1181,39 MiB memoria GPU dedicata**, **1,5 MiB condivisa**, picchi di categorie
separate. Non sommare questi valori al commit privato né confonderli con
la capacità VRAM della scheda. Log iniziali: `flash attn = 0`, `gpu_device = 0`,
modello su Vulkan0 e `using Vulkan0 backend`; il VAD usa separatamente CPU.

## Limiti

Solo 12 frasi lette, filtro durata sbilanciato 11 voci etichettate femminili
e una maschile; niente conversazione spontanea, sovrapposizioni o audio
Discord reale. Stress Opus/volume simulato. Nessun microfono, cattura,
riproduzione, screenshot o controllo del desktop. Non certifica accuratezza
o stabilità prolungata durante una call. File-only del binario integrato,
hash e suite Rust sono registrati separatamente in [VALIDATION.json](VALIDATION.json).

Ricerca locale congelata: report SHA-256
`e372eadcccaff8fb2bbc6c48f23de29adcc2c539622df67f7945cf10f059fff9`.
I record grezzi privati sono conservati nello workspace, esclusi dai pacchetti.
