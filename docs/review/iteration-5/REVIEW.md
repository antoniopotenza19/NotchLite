# Revisione indipendente — navigazione e Rose 0.1.5

Data: 2026-11-23. Revisore: agente security_audit, separato dall'implementatore. Ruolo read-only sul prodotto; unico file scritto dal revisore: questo report. Metodo: AGENTS.md, skill locale Ponytail Review e checklist upstream, lettura di sorgenti, chiamanti, diff e test. Il grafo disponibile era ancora quello dell'iterazione precedente: per questo cambiamento sono stati seguiti direttamente i riferimenti del codice.

**Cosa cambia:** la notch mostra una barra di icone delle pagine effettivamente abilitate, con titolo della sezione corrente o indicata dal puntatore. Aggiunge Ctrl+Tab/Ctrl+Shift+Tab quando non si sta modificando un campo, e il quarto preset Rose. Non aggiunge un servizio di trasferimento o pairing con il telefono.

**Esito statico:** nessun difetto concreto che richieda correzione individuato nei percorsi controllati. La conclusione riguarda questa modifica; non equivale a una prova delle interazioni o della resa su Windows.

## Barra: disegno, clic e fallback

- `src/navigation.rs:23` produce un layout soltanto per 1–12 schede, coordinate finite, altezza sufficiente e spazio per icone e stato. Il lavoro è delimitato e usa dati sullo stack; non legge file, non alloca contenitori e non avvia attività.
- `src/shell.rs:598` e `:814` costruiscono lo stesso Layout a partire da `expanded_panel()` e numero di pagine; il disegno usa `tab(i)`, mentre `hit()` interroga gli stessi rettangoli. `Rect::contains` (`src/gfx.rs:72`) esclude bordo finale, spaziatura e padding. Non viene selezionata una scheda vicina cliccando nello spazio tra icone.
- `page_ids` e `pages` provengono dallo stesso `unzip` durante la ricostruzione (`src/shell.rs:306`); `pages::build` include soltanto i moduli abilitati, seguiti da Settings (`src/pages/mod.rs:181`). Le icone usano quegli ID, non un elenco statico di funzioni potenziali. Nessuna icona Phone o finta schermata connessa è stata aggiunta.
- `arrow_hit` e `dot_hit` (`src/shell.rs:686`, `:702`) sono disabilitati quando il layout della barra è disponibile, come il corrispondente ramo di disegno. Se il layout non entra, disegno e input mantengono entrambi la navigazione precedente.

Tentativo di confutazione: un layout valido solo nel disegno avrebbe lasciato frecce o pallini invisibili cliccabili. I tre punti usano invece la stessa funzione e gli stessi argomenti. Con l'attuale massimo di 11 pagine la barra entra nel pannello largo 420 DIP; i test coprono anche il limite di 12 e il fallback sotto la larghezza minima. La presenza fisica e riconoscibilità dei glifi Fluent restano da vedere sul sistema reale.

## Tastiera, selezione e ricostruzione

`src/shell.rs:936` accetta Ctrl+Tab solo quando il pannello è espanso, `keyboard` è falso e la pagina non è in editing. In quel caso lo spostamento usa resto euclideo per tornare dall'ultima alla prima pagina e viceversa. Il conteggio non può essere zero dopo `pages::build`, perché Settings viene sempre aggiunta.

Tentativo di confutazione: una scorciatoia intercettata durante un editor avrebbe sottratto input al campo. I controlli precedono la scorciatoia e i due editor attuali dichiarano il proprio stato tramite `editing()` (`todo.rs:352`, `calendar.rs:502`). Il clic su un'altra scheda passa per `go_to`, che conclude l'editor prima di cambiare destinatario dell'input (`src/shell.rs:499`). Non è stato provato l'invio reale dei messaggi tastiera di Windows.

`nav_hover` viene eliminato dopo ricostruzione delle pagine, uscita del puntatore, chiusura del pannello e cambio sezione (`src/shell.rs:315`, `:438`, `:499`, `:785`). Il titolo applica inoltre un limite esplicito all'indice hover (`:555`). Non è emerso un indice obsoleto usabile dopo il cambio dei moduli abilitati.

## Preset Rose

`src/theme.rs:46` conserva i valori Original=0, Graphite=1, Midnight=2 e aggiunge Rose=3. Parsing, identificatore persistito, etichetta, ciclo e selezione della palette includono il nuovo valore; chiavi assenti o sconosciute conservano Original. La selezione resta un accesso atomico alla palette statica, senza nuovo I/O, lock o worker.

I tre test del tema includono tutti e quattro i preset: round-trip/fallback, ciclo completo, contrasto del testo su pannello e hover e testo inverso sui controlli selezionati. Il test delle Settings comprende Rose nel ciclo e confronta anche le altre preferenze. Tentativo di confutazione: un'aggiunta soltanto all'enum avrebbe potuto essere salvata ma non selezionata, oppure saltata nel ciclo; i relativi match e test coprono entrambi i percorsi. Questi controlli matematici non certificano contrasto di ogni immagine, stato transitorio o pixel renderizzato.

## Trasferimento browser via QR: perimetro separato

Il riferimento osservato è un video dimostrativo di trasferimento file nel browser del telefono, per Android e iPhone. Un companion APK o un bridge KDE Connect non rappresentano lo stesso flusso. Non è stata identificata una repository originale riutilizzabile e non si attribuisce al video una licenza o un'implementazione verificata.

Questa iterazione non contiene listener, provider di pairing, apertura di porte, identità del telefono, QR operativo o promessa di trasferimento sicuro. Una futura implementazione deve risolvere autenticazione del server, origine attendibile del contenuto web e trasporto dei file prima di presentare un collegamento come sicuro. Un QR con indirizzo HTTP LAN non fornisce queste proprietà. Questa è una nota di confine del lavoro, non un audit di un backend esistente.

Riletti anche `docs/PHONE-SHARE.md`, `docs/MODERN-NAVIGATION.md` e l'aggiornamento finale di `docs/APPEARANCE.md`: distinguono specifica e funzione disponibile, companion nativo e browser, schema illustrativo offline e screenshot della GUI. La documentazione dell'aspetto include Rose e il ciclo dei quattro preset. Il documento del trasferimento lascia esplicitamente aperto il trust HTTPS e non propone di aggirare avvisi TLS o installare certificati nel normale onboarding.

## Verifiche e limiti

Il revisore non ha eseguito build, test, app, strumenti desktop, scanner, socket o pairing. Il coordinatore ha completato la validazione finale: **40 test passati, zero falliti**, e build release riuscita. Il revisore ha letto le righe finali dei suoi log locali `work/iteration-5/unit-tests.log` e `work/iteration-5/release-build.log` (quest'ultimo termina con `Finished release profile` in 12,60 secondi). Le esecuzioni restano attribuite al coordinatore; questa lettura non è una seconda esecuzione indipendente. Il coordinatore conferma inoltre che nessun sorgente Rust è cambiato dopo lo snapshot qui riportato.

I test della navigazione verificano geometria, limiti/fallback, hit di ogni icona e rifiuto di padding/spazi, input non finiti, scelta dei tasti e glifi statici. Non aprono finestre e non misurano RAM. La proprietà senza allocazioni è circoscritta al modulo navigation: shell e renderer conservano le loro stringhe e chiamate di disegno, quindi non si dichiara zero allocazioni dell'intero frame.

Non verificati: resa, dimensioni apparenti delle icone e focus reali, DPI, fullscreen, Win+D, lettori di schermo, comportamento dei puntatori durante le animazioni, RAM/GPU e latenza. Resta valido il vincolo dell'utente di procedere senza desktop e senza avviare l'app.

## Snapshot dei sorgenti riletti

SHA-256 dei byte locali, prima di eventuali conversioni di fine riga:

| File | SHA-256 |
|---|---|
| `src/navigation.rs` | `585cffd4163f5c0745d98d83888046b238b5a1297e86373993c44e98447a4950` |
| `src/shell.rs` | `461045380c54ccd6b0efd6b288cc91b9d744787b0e1246ad59de83be754dcf08` |
| `src/theme.rs` | `e6eeabde7e910c89a489f6ada872c59c4e3c95bd5f82a82069c1593836974011` |
| `src/pages/settings.rs` | `c7ec490fdf43d102795c7afd82c6ef0b89fdfb25ad0f64cd024614e1c684ae80` |
| `src/main.rs` | `3fb05fe1a34cb8f79ade65b02286f8755051bd2b58250c6fad172417acb9de2e` |

**Verdetto:** pubblicabile come preview sperimentale della nuova UI nel perimetro revisionato; le verifiche CLI del coordinatore sono completate. Nessuna equivalenza con un collaudo GUI o con un trasferimento telefono disponibile.
