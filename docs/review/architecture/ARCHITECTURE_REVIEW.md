# Revisione dell'architettura e della preparazione alla pubblicazione

> Snapshot prima delle correzioni dell'audit, conservato come evidenza storica. Per il codice successivo e la suite a 15 test leggere POST_PATCH_NOTE.md e ../FINAL_REVIEW.md.

Revisione statica del 23 novembre 2026. Perimetro: i 24 file Rust di src/ (9.412 righe nel campione). Nessun codice applicativo eseguito in questa revisione. Il grafo è uno strumento di navigazione, non una prova di sicurezza.

## Provenienza verificata

La base dichiarata è Ada Notch, revisione 5a1e17f040672a71685872b1b234e01d02a47bb5, con licenza Apache-2.0 e NOTICE conservati. Confronto byte per byte con l'archivio di quella revisione:

- 20 file Rust ereditati sono identici.
- 3 file ereditati sono modificati: main.rs, shell.rs e store.rs.
- 1 file è nuovo: profile.rs.

Questo campione supporta la descrizione «versione modificata/derivata da Ada». Non supporta «riscrittura completa» né l'attribuzione esclusiva di tutto il codice a NotchLite. I dettagli e gli hash dei file esaminati sono in SOURCE_SNAPSHOT.json.

## Mappa dei componenti

| Componente | Evidenza | Responsabilità |
|---|---|---|
| Avvio e profilo | src/main.rs:22; src/profile.rs:11 | Istanza unica, profilo iniziale, avvio finestra |
| Shell Windows | src/shell.rs:1071; src/shell.rs:691 | Finestra overlay, input, animazione, tick generale |
| Contratto delle pagine | src/pages/mod.rs:53; src/pages/mod.rs:180 | Interfaccia Page, costruzione dei moduli abilitati |
| Worker | src/worker.rs:22; src/worker.rs:88 | Canali/Win32 PostMessage, thread di background |
| Grafica | src/gfx.rs | Direct2D/DirectComposition e risorse grafiche |
| Dati | src/store.rs | File locali di impostazioni, liste e cache |
| Rete | src/http.rs | Wrapper WinRT HTTP usato dai moduli opzionali |

La struttura a pagine permette di non costruire i moduli disabilitati all'avvio. Non costituisce un confine di sicurezza o un isolamento dei consumi: i moduli condividono processo, heap, thread UI e accesso Windows. Il tick della shell visita ogni pagina attiva una volta al secondo (shell.rs:691); ogni wake visita poll() di tutte le pagine attive (shell.rs:709).

## Lacune da trasformare in requisiti

### A-01 — Lifecycle dei moduli non verificabile in modo uniforme

Page espone tick/poll/draw e gli handler, ma non un protocollo comune stop/join o uno stato dei worker (pages/mod.rs:53-127). worker::spawn restituisce () e scarta il JoinHandle (worker.rs:88-101). La terminazione deve quindi essere verificata separatamente nelle implementazioni dei singoli moduli. Questo è un limite architetturale, non la dimostrazione che tutti i worker perdano risorse.

Correzione minima proposta: un contratto di cancellazione e ownership dei worker; test di enable/disable ripetuto con attesa di completamento e misure thread/handle. Non riscrivere l'interfaccia completa prima di aver corretto i casi concreti confermati dalle altre revisioni.

### A-02 — I test coprono poco le funzioni di uso quotidiano

Nel campione esistono 10 test in soli tre file: 7 nel modulo calcio, 2 nel drag-and-drop/URL e 1 nel profilo iniziale. Non sono presenti test dedicati per shell, gestione delle impostazioni, persistenza, grafica, focus timer, lifecycle media o meteo. I test descritti come passati nella validazione precedente non dimostrano correttezza di questi percorsi.

Correzione minima proposta: separare dalla UI le regole pure con effetti importanti (cancellazione, parse/persistenza, limiti di input, scadenze) e aggiungere casi di errore concreti. Per hover, DPI, keyboard focus e drag-and-drop resta necessario un collaudo Windows; un test che replica le costanti non sostituisce tale collaudo.

### A-03 — Budget RAM e responsiveness richiedono scenari attivi

Il campione a riposo esistente non copre la musica, il pannello aperto, i download dei moduli e i cicli enable/disable. L'interfaccia comune permette lavoro in tick()/poll() senza un limite temporale esplicito; ogni implementazione va controllata per I/O sul thread UI e code non limitate. Non è misurata qui una regressione.

Correzione minima proposta: misure separate di memoria privata, working set, thread/handle e latenza durante funzioni attive; regressioni legate al modulo che le causa. Conservare come obiettivo condizionato il budget di base, evitando di promettere lo stesso valore con tutti i moduli attivi.

### A-04 — Readiness per GitHub

README e NOTICE devono mantenere in apertura la derivazione, il carattere sperimentale e i limiti delle prove. Il modulo grafico e il ciclo Win32 restano in larga parte codice upstream. Il viewer Graphify è documentazione, non un componente dell'app: non va caricato o eseguito nel processo NotchLite. Registrare licenze e provenienza degli eventuali componenti che saranno davvero integrati.

## Tooling e limiti della mappa

Usato Graphify (distribuzione già installata graphifyy 0.8.26), repository originale dichiarato nei metadati: https://github.com/safishamsi/graphify. Licenza MIT. Non è disponibile direct_url.json, quindi non viene attestata una revisione Git del tool. Non è stato installato alcun pacchetto e i sorgenti non sono stati inviati a servizi.

Risultato: 853 nodi, 1.983 archi nel grafo consolidato, 22 comunità. L'hub più connesso è App (61 collegamenti), seguito da MediaPage (32) e ShelfPage (31). I valori di coesione sono riportati senza arrotondamenti simbolici in METADATA.json. Le associazioni tra simboli omonimi e il dynamic dispatch richiedono conferma sui sorgenti; il clustering non dimostra separazione a runtime.

La pipeline AST ha usato 0 token aggiuntivi LLM. Il benchmark automatico di Graphify stima 7,8 volte meno token per query rispetto all'invio dell'intero corpus; le domande generiche del benchmark non sono prove di qualità delle risposte né prestazioni del programma.

File prodotti: graph.json, graph.html, GRAPH_REPORT.md, METADATA.json e SOURCE_SNAPSHOT.json. Il viewer HTML usa vis-network 9.1.6 da unpkg quando viene aperto; la generazione del grafo non ha richiesto accesso a quel CDN. Le informazioni pubblicabili usano percorsi relativi al repository.
