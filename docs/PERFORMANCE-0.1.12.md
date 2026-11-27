# Risorse NotchLite 0.1.12 — 27 novembre 2026

Binario GNU SHA-256 `57302cea862737f4d900390c3781dfd604a12d28741e711654bd851a0b495ebf` installato e avviato via CLI,
senza screenshot o input del desktop. Preferenze conservate. Metadati OS:
notch sul secondario e un pannello call visibile, nessuna intersezione.
Non è una verifica visiva del testo o dell'interazione.

Campione inattivo di 20.199 secondi dopo l'avvio:
working set massimo **61.75 MiB**, commit
privato massimo **51.87 MiB**. CPU campionata
**0,0%**, sotto la risoluzione del contatore nell'intervallo, non promessa
di consumo nullo. 8 processori logici, fino a 20 thread e 1116 handle.
Non sono attivi ascolto o Whisper. Esclude server Codex e memoria GPU.
Campione breve, diverso dai profili storici: non dimostra una regressione
o un miglioramento generale della memoria.

Durante l'inferenza Turbo il costo separato è più alto:
[campionamenti CPU/GPU e accuratezza](review/iteration-13/TRANSCRIPTION.md).
Il processo Whisper termina per segmento; il modello non rimane residente
quando l'ascolto è spento. Nessuna misura dell'intera app durante una vera call.

[Test, hash e limiti](review/iteration-13/VALIDATION.json).
