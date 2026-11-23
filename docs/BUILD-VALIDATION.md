# Verifiche della prima build portatile

Documento storico della build precedente alla revisione. Per la build corretta, compilata e verificata con 15 test, vedere [VALIDATION.json](review/VALIDATION.json) e [il riepilogo](review/SUMMARY.md). I consumi qui riportati non sono stati rimisurati dopo le correzioni.

Data: 23 novembre 2026. Sorgenti del fork locale NotchLite, derivati da Ada alla revisione indicata in PROVENANCE.json.

## Compilazione e test

- Rust 1.100.0, commit 0ef1bc386059596dada8fb65b50425aab2ea637a.
- Cargo 1.100.0; target x86_64-pc-windows-gnu.
- Componenti standalone rustc, cargo, rust-std e rust-mingw dai server ufficiali static.rust-lang.org. SHA-256 dei pacchetti confrontati con il manifest ufficiale.
- Toolchain e cache nella cartella work del progetto; nessun installer di sistema eseguito.
- cargo test --locked: 10 test passati, 0 falliti. Coprono profilo iniziale/preservazione impostazioni, scorciatoie URL e parser/logica del modulo calcio.
- cargo build --release --locked: riuscito. Cinque avvisi per codice inutilizzato ereditato; nessun errore.
- notchlite.exe: 756736 byte.
- SHA-256: FEA7E68C35874112C509282887DC90EA8417EA568A9DE8216BC6ACE40BD1B5E7.

## Avvio

Avvio dal pacchetto portatile con il PATH ordinario della sessione, senza aggiungere il compilatore. Processo attivo e rispondente; titolo finestra NotchLite e handle principale presenti. Profilo di prova isolato sotto work/app-smoke: impostazioni iniziali corrette, senza modificare quelle reali dell'utente.

Il controllo nativo dell'interfaccia non ha restituito una finestra selezionabile per questo overlay. Non sono quindi verificati visivamente espansione al passaggio del mouse, navigazione, drag-and-drop e resa grafica. Il processo di prova è stato chiuso dopo le misure.

## Primo campione di consumi

15 secondi di stabilizzazione, poi 60 campioni su 60,596 secondi. Profilo iniziale: deposito file, attività e focus; altri moduli disabilitati. Windows 10.0.26300.0, 8 processori logici.

- Memoria privata media/massimo campionato: 36,938 MiB.
- Working set medio/massimo campionato: 48,777 MiB.
- Nessun incremento del contatore CPU osservato nel campione. Il risultato arrotondato è 0%, entro la risoluzione del contatore: non dimostra consumo sempre nullo.
- Massimi campionati: 14 thread, 1050 handle.
- Una lettura separata delle connessioni TCP del processo ha restituito zero connessioni. Non è una cattura completa del traffico.

Il campione pubblico sintetizzato è in benchmarks/base-idle.json, senza identificativi del processo. CSV e JSON grezzi restano nelle evidenze locali e non sono pubblicati.

Queste misure riguardano solo il processo inattivo durante questa prova. Non includono memoria GPU, processi esterni, carico con musica, uso interattivo o tutti i moduli attivi. Un singolo intervallo non esclude perdite di risorse nel lungo periodo.
