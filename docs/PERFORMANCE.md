# Consumi e prove

La build 0.1.5 e l'orologio non sono stati avviati né misurati: le verifiche del prodotto sono limitate alla riga di comando per richiesta dell'utente. Le ultime misure restano quelle della 0.1.2 in [PERFORMANCE-0.1.2.md](PERFORMANCE-0.1.2.md): due scenari inattivi, 60 campioni ciascuno, profilo base. Il working set medio era 43,6 MiB in background e 42,8 MiB con il pannello aperto. Il budget di 40 MiB di memoria privata non era raggiunto in tutti gli scenari. Non attribuire questi valori alla 0.1.5, all'orologio acceso o a un futuro backend telefono.

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

Accessibilità, DPI/multi-monitor, rete dei moduli opzionali, fullscreen reale, uso prolungato, memoria GPU, orologio e latenza. I risultati CLI correnti sono nel [report di validazione](review/iteration-5/VALIDATION.json). Apertura e navigazione erano state osservate nella 0.1.2; la GUI della 0.1.5 non è stata avviata. L'avvio del processo e il consumo inattivo sono verifiche distinte dal funzionamento completo dell'interfaccia.
