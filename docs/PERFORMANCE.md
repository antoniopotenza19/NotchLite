# Consumi e prove

I valori nella tabella seguente sono budget iniziali, non risultati misurati. Il campione sintetizzato della prima build, separato dagli obiettivi, si trova in benchmarks/base-idle.json ed è descritto in BUILD-VALIDATION.md. Non è una misura della build successiva alle correzioni.

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

Accessibilità, DPI/multi-monitor, interazioni visive, rete dei moduli opzionali, fullscreen, uso prolungato, memoria GPU e latenza. Compilazione release e 10 test Rust sono passati. L'avvio del processo e il consumo inattivo sono verifiche distinte dal funzionamento completo dell'interfaccia.
