# Funzioni e sviluppo graduale

Riferimento funzionale: [sito ufficiale NotchPixel](https://notchpixel.com/), consultato il 23 novembre 2026. Osservazione delle descrizioni pubbliche; app NotchPixel non provata. Le funzioni menzionate qui costituiscono requisiti del nostro prodotto, non una promessa di compatibilità completa.

| Funzione desiderata | Stato nella base locale | Passo necessario |
|---|---|---|
| Notch compatta ed espansione al passaggio del mouse | Codice ereditato | Provare input, focus e schermi |
| Controlli musica e testi sincronizzati | Codice ereditato, spento nel profilo iniziale | Provare sessioni media, limitare cache e worker |
| Deposito file trascinabili | Codice ereditato, attivo | Provare Explorer, browser e file mancanti |
| Timer, attività, calendario | Codice ereditato; calendario spento | Aggiungere promemoria ricorrenti, snooze e persistenza |
| Meteo, orologio e batterie | Parzialmente ereditato | Provare errori rete e batterie periferiche |
| Cronologia appunti con schede e pin | Da implementare | Limiti memoria/disco e regole contenuti |
| Invio file al telefono | Da implementare | Trasferimento LAN avviato dall'utente |
| PC Deck e telecomando telefono | Da implementare | Pairing e companion app separata |
| Barra alternativa con Start, tray e app | Da implementare | Modulo dedicato e ripristino affidabile |
| FPS, temperature e statistiche in gioco | Solo statistiche CPU/RAM ereditate | Sorgenti dati e prove con giochi diversi |
| Muto microfono e timer chiamata | Da implementare | Integrazione audio Windows |
| Widget indipendenti sul desktop | Da implementare | Lifecycle e limiti per finestra |
| Stato agenti AI | Lettura utilizzo Claude locale ereditata | Stati operativi Codex e altri agenti |
| Cronologia notifiche | Anteprime ereditate | Provare accessi, archivio e limiti |
| Chat Twitch/YouTube e stato OBS | Da implementare | API ufficiali e autenticazione |
| Volume, luminosità, Wi-Fi e altri controlli | Non completi nella base | Azioni native e alternative per hardware |
| Pannello con siti web | Da implementare | Componente caricato solo quando richiesto |
| Temi e liquid glass | Grafica nativa ereditata, senza equivalenza completa | Tema originale, effetto opzionale con budget GPU |
| Lingue e accessibilità | Da sviluppare | Italiano, tastiera, contrasto e UI Automation |

Le fasi comprendono tutte le funzioni sopra; l'ordine serve a mantenere misurabile il costo di ciascuna.

Priorità aggiornata: il risultato visivo completo del desktop viene prima delle integrazioni telefono/streaming. La preview attuale copre soprattutto la notch. Vedere [DESKTOP-EXPERIENCE.md](DESKTOP-EXPERIENCE.md) per barra, sfondo, orologio, widget e profili Leggero/Vetro: sono obiettivi ancora da implementare, non funzioni già presenti.

## Fase 0 — base e misure

Compilare il fork, eseguire i test, verificare il profilo iniziale e registrare i consumi. Criterio: avvio, espansione, chiusura e uscita affidabili; dati separati da Ada; nessuna impostazione utente sovrascritta.

## Fase 1 — uso quotidiano

Musica, volume, microfono, avvio rapido app, deposito file, appunti e timer. Interfaccia con accesso diretto alle funzioni preferite, comandi grandi e impostazioni separate. Criterio: ogni comando funziona con tastiera e mouse; disabilitare un modulo ne interrompe anche le attività in background, dopo gli eventuali timeout documentati.

## Fase 2 — organizzazione e desktop

Questa fase diventa prioritaria dopo la base di apertura/affidabilità: preset coordinato, sfondo statico, orologio e widget desktop, poi barra/dock. Vetro e sfondo animato saranno opzioni misurate separatamente.

Promemoria ricorrenti, calendario, meteo, notifiche, widget e monitor multipli. Criterio: prove al 100/125/150/200% DPI, sleep/resume, monitor scollegato, desktop remoto e comportamento a schermo intero.

## Fase 3 — telefono e barra alternativa

Condivisione LAN, telecomando e taskbar. Criterio: attivazione esplicita, connessione riconoscibile e ripristino della barra di Windows se l'app termina.

## Fase 4 — integrazioni e funzioni avanzate

Statistiche gaming, agenti, OBS, chat, siti e temi. Ogni integrazione ha un proprio budget e può restare disattivata. La piena compatibilità con fullscreen esclusivo e anti-cheat richiede verifiche specifiche; non è garantita.

## Scelte architetturali

Riutilizzare le integrazioni già funzionanti prima di riscriverle. Profilare i percorsi costosi. Spostare rete e operazioni lente fuori dal thread UI. Aggiungere un lifecycle esplicito start/stop a ogni modulo e cache limitate. Aprire un eventuale componente web solo su richiesta e misurarne separatamente i processi; offrire sempre una configurazione leggera.
