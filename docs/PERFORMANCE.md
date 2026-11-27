# Consumi e prove

La 0.1.11 ha un nuovo [campione UI inattiva](PERFORMANCE-0.1.11.md): venti secondi con notch e pannello call ampio, senza avvio della trascrizione. Le misure seguenti delle versioni precedenti restano storiche e non sono confronti controllati.

La GUI 0.1.10 e l'orologio non sono stati avviati né misurati: il collegamento al daemon Codex è stato provato in modalità headless. Il daemon è un processo separato e aggiunge un costo RAM proprio; non è incluso nelle misure storiche della notch. Le ultime misure restano quelle della 0.1.2 in [PERFORMANCE-0.1.2.md](PERFORMANCE-0.1.2.md): due scenari inattivi, 60 campioni ciascuno, profilo base. Il working set medio era 43,6 MiB in background e 42,8 MiB con il pannello aperto. Il budget di 40 MiB di memoria privata non era raggiunto in tutti gli scenari. Non attribuire questi valori alla 0.1.10, all'orologio acceso o al backend telefono.

I valori nella tabella seguente sono budget iniziali, non risultati misurati. Il campione della prima build in benchmarks/base-idle.json e BUILD-VALIDATION.md resta storico.

| Metrica | Obiettivo del profilo base |
|---|---|
| Memoria privata del processo, stabilizzata | provare a restare entro 40 MiB |
| CPU media, notch inattiva | inferiore a 0,1% della capacità totale del PC |
| Risposta visiva al comando | inizio risposta entro 100 ms |
| Risorse dopo uso ripetuto | nessuna crescita persistente non spiegata |

Non sommare né confondere memoria privata, working set, dimensione EXE e memoria GPU. Riportare tutte le voci disponibili. Il budget base non vale automaticamente per tutti i moduli attivati, per un browser incorporato o per la decodifica multimediale.

## Protocollo ripetibile

1. Build release con lockfile; annotare revisione, Windows, CPU/GPU, driver, DPI, monitor e moduli attivi.
2. Primo avvio e secondo avvio: verificare che le preferenze siano preservate.
3. Notch chiusa e senza attività: 15 secondi di stabilizzazione, 60 secondi di campionamento.
4. Ripetere con musica in riproduzione, poi pannello aperto; riportare i risultati separatamente.
5. Aprire/chiudere 100 volte, trascinare file e cambiare traccia. Ripetere il campionamento dopo il rilascio delle risorse.
6. Abilitare un modulo per volta, poi disabilitarlo. Verificare che worker, timer, file e socket si arrestino entro tempi documentati.
7. Ripetere dopo sospensione e riattivazione e con un'altra scala DPI.

Lo script Measure-Notch.ps1 registra private bytes e working set correnti; i massimi nel JSON sono massimi campionati nell'intervallo. CPU è la differenza del tempo CPU del processo divisa per tempo reale e processori logici. Non è il consumo elettrico. Il JSON riporta sia percentuale normalizzata sia equivalente su un core.

Per la latenza usare tracce ETW o una ripresa con timestamp: un controllo del file EXE o un cronometro intorno a Start-Process non misura il primo frame visibile. Per GPU e processi aggiuntivi usare strumenti Windows adatti e un report separato.

## Verifiche non ancora eseguite

Accessibilità, DPI/multi-monitor, rete dei moduli opzionali, fullscreen reale, uso prolungato, memoria GPU, orologio e latenza. I risultati CLI correnti sono nel [report di validazione](review/iteration-10/VALIDATION.json). Apertura e navigazione erano state osservate nella 0.1.2; la GUI della 0.1.8 non è stata avviata. L'avvio del processo e il consumo inattivo sono verifiche distinte dal funzionamento completo dell'interfaccia.

## Motore di trascrizione opzionale

Su un WAV pubblico inglese di sei secondi, il processo separato whisper-cli CPU con tiny-q5_1, due thread e audio context 512 ha impiegato 309 ms. Picchi campionati: **96,12 MiB working set** e **605,76 MiB memoria privata impegnata**. La diagnostica del prodotto ha impiegato 281 ms. Queste misure riguardano un file e il solo Whisper: non la GUI, WASAPI, una call italiana o la latenza completa. Il motore è avviato per segmento e non caricato quando la trascrizione è spenta. Il budget del profilo base non si applica alla trascrizione attiva. Vedere [CALL-TRANSCRIPTION.md](CALL-TRANSCRIPTION.md).

Le impostazioni tray/cattura della 0.1.9 hanno test puri e diagnostica OS; non è stato misurato il loro costo in GUI. [Validazione corrente](review/iteration-10/VALIDATION.json).

## Profilo call 0.1.10

Base Q5_1 e Silero VAD privilegiano la qualità rispetto al vecchio Tiny con contesto ridotto. Il costo del motore avviato per segmento e del daemon Codex è separato dal profilo base della notch. Le misure Whisper precedenti restano storiche e non descrivono questo profilo. Nessuna nuova misura della GUI durante una call reale. [Verifiche correnti](review/iteration-11/VALIDATION.json).
