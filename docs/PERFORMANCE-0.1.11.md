# Misura UI 0.1.11

Campione di 20 secondi dopo cinque secondi di stabilizzazione, build GNU release. Graphite, notch visibile sul secondario e pannello call ampio 640×900 DIP. App appena riavviata senza avviare la trascrizione; stato “Ascolto spento” rilevato al controllo finale.

| Metrica | Valore |
| --- | --- |
| Working set medio | 45.92 MiB |
| Working set massimo campionato | 45.92 MiB |
| Memoria privata media | 40.52 MiB |
| Memoria privata massima campionata | 40.52 MiB |
| CPU media normalizzata sul PC | 0.0289% |
| CPU equivalente su un core | 0.2316% |
| Thread massimi campionati | 20 |
| Handle massimi campionati | 1122 |

Misura breve di un singolo processo. Non comprende memoria GPU, DWM, daemon Codex, motore Whisper o call reale. Non è un confronto controllato con la 0.1.10 né prova di assenza di crescita dopo molti trascinamenti. Il budget base di 40 MiB privati non è raggiunto da questa misurazione con pannello call abilitato. Le misure della 0.1.2 restano storiche.

Script: `tools/Measure-Notch.ps1`, `Seconds=20`, `WarmupSeconds=5`. Output raw mantenuto locale; riepilogo e impronta nel [record](review/iteration-12/VALIDATION.json). Nessuno screenshot o input desktop inviato per la misura.
