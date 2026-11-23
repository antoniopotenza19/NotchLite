# Revisione indipendente — tema 0.1.3

La modifica aggiunge tre palette scure native, selezionabili da Settings e conservate nel profilo. Le funzioni delle pagine restano quelle esistenti; cambia la risoluzione dei colori, senza aggiungere un motore web o nuove dipendenze.

## Perimetro e metodo

Revisore indipendente: security_audit. Applicati AGENTS.md e la skill locale Ponytail Review con la checklist originale fissata alla revisione ebd8654c2c627f721eb8f9c0d1bd4090a29f9e10.

Ispezionati theme.rs, profile.rs, il flusso Settings → store → ShellCmd → reload, i chiamanti dei token e il riuso dei brush in gfx.rs. Le conversioni pages/*, escluso Settings e la successiva correzione del calendario, sono state confrontate con HEAD dopo normalizzazione dei nomi dei token: nessun cambiamento di logica oltre alle sostituzioni meccaniche e agli avvisi di provenienza.

Il grafo disponibile aveva hash non corrispondenti per i file modificati durante la prima ispezione; sono stati quindi seguiti direttamente sorgenti e chiamanti. Non è stato rigenerato dal revisore.

## Finding risolti durante la revisione

### 1. Contrasto del giorno selezionato nel calendario

Caso concreto: sui nuovi preset la lettera del giorno selezionato, disegnata a 9 DIP con bg al 60%, veniva compositata sopra accent o text. Il calcolo statico dava Graphite 3,796:1 / 4,360:1 e Midnight 3,483:1 / 4,472:1; Original superava 4,5:1 in entrambi i casi.

Tentativo di confutazione: il test iniziale copriva panel e hover card, ma non queste due superfici invertite; il dato deriva dai colori e dal ruolo effettivamente usato, non da una preferenza stilistica né da una schermata osservata.

Correzione verificata: calendar.rs:363–379 usa bg opaco per la lettera; theme.rs:195–198 controlla anche il contrasto di bg sopra accent/text. Finding chiuso nel sorgente; resa visiva non provata.

### 2. Errore di rilettura convertito in impostazioni vuote

Caso concreto: dopo il salvataggio del preset Settings richiedeva un ReloadSettings. Se quella seconda lettura falliva, read_kv trasformava l'errore in mappa vuota: il reload poteva applicare Original, mascotte attiva e lista disabled vuota, abilitando moduli che l'utente non aveva selezionato.

Tentativo di confutazione: la lettura preliminare e il test di scrittura fallita proteggevano il selettore, ma non la seconda lettura nella shell. Il difetto era nel percorso di errore preesistente, ora raggiunto anche dal selettore nuovo; non si afferma che sia stato osservato su questo PC.

Correzione verificata: store.rs:44–55 espone try_read_kv; shell.rs:272–304 usa ? prima di qualsiasi mutazione. Lo startup propaga l'errore (shell.rs:1302). Nei reload successivi lo stato attivo resta immutato; il limite del feedback viene descritto in APPEARANCE.md. Finding chiuso nel sorgente.

### 3. Il toggle dal tray poteva ancora perdere preferenze dopo un errore di lettura

Caso concreto: set_setting leggeva con il helper che mascherava gli errori e poteva poi salvare una mappa contenente solo la chiave Mascot/Minimal. Appearance, disabled e le altre preferenze potevano essere perse.

Tentativo di confutazione: la correzione del solo reload non proteggeva il writer del tray; il salvataggio poteva già avere rimpiazzato il contenuto.

Correzione verificata: shell.rs:307–312 usa try_read_kv e procede al reload soltanto dopo una scrittura riuscita. Finding chiuso nel sorgente; il feedback degli errori del tray non è stato ampliato da questa modifica.

## Controlli senza nuovi finding

- Preset assente, vuoto o sconosciuto ricade su Original; solo gli identificatori interni selezionano Graphite e Midnight. Nessun valore configura file esterni, shader o URL.
- Il primo profilo nuovo aggiunge appearance=graphite; un profilo esistente resta preservato dal percorso di inizializzazione già presente.
- La palette è immutabile. AtomicU8 pubblica soltanto il suo ID: Relaxed è adeguato, perché non deve sincronizzare altri dati mutabili. La scrittura del preset avviene nel thread UI; i worker possono leggere un accento per le notifiche.
- Il cambio solo-aspetto non ricostruisce pagine quando disabled resta uguale. Il timer focus e gli altri moduli non vengono riavviati per il solo cambio di colore.
- I getter colore usano palette statiche e valori Color copiati. Non fanno I/O, non avviano thread e non introducono allocazioni heap nel rendering. Il brush esistente viene aggiornato con SetColor per ogni disegno.
- Cargo.toml e Cargo.lock cambiano soltanto versione del pacchetto: nessuna dipendenza aggiunta.
- Le opacità secondarie più alte anche su Original sono una scelta esplicita documentata, non una promessa di riprodurre ogni vecchio valore alpha. Le anteprime già accodate conservano l'accento catturato: limite documentato.

## Verifiche e limiti

Questo revisore non ha eseguito compilazioni, scanner, l'app o test sul desktop. Ha controllato il diff, il flusso sorgente, i test introdotti e il calcolo statico del contrasto; git diff --check non ha riportato errori. Il coordinatore ha comunicato 27/27 test normali sintetici passati; i risultati di build e packaging appartengono alla sua validazione.

Nessuna finestra aperta, screenshot, modifica del tema Windows o lettura delle impostazioni reali del PC. Resa grafica, clic del selettore, DPI, animazioni, accessibilità completa e RAM/CPU della 0.1.3 non sono stati provati. I valori della 0.1.2 restano storici.

## Identificatori del sorgente esaminato

Base Git: 526de1fa866bff3697c580b2a2438a1c169f8f08, più le modifiche di questa iterazione.

| File | SHA-256 |
|---|---|
| src/theme.rs | e34a45d67e31d0874596d97e916f76472453db284d81e726ea7212639651f035 |
| src/pages/settings.rs | 36ce3119e753a680429d94216303319a82b9cecddcfa8ecd054694019dc96730 |
| src/shell.rs | 703c800d4b57a16508e0aaE20454a9298545c59a4b6c5d0f7b1a2c66f728c4b4 |
| src/profile.rs | 6f1958d68a845d0ba4a49b623bdc6ceed2527223fab4c06e9017d67ec06f1252 |
| src/pages/calendar.rs | e209ed8f117fd87427a3ed0d4d9f577a21a8bdb55dee85304743759633554208 |
| src/store.rs | a6ee9e04817cdd5b8c9bab89311e0399bca759dcd2e9eb9a53140222d9102f34 |

Verdetto: nessun finding bloccante residuo nel perimetro esaminato, dopo le tre correzioni. Adeguato a un'anteprima sperimentale con i limiti sopra; non è un'approvazione della resa GUI né una certificazione dell'intero prodotto.
