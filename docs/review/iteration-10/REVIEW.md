# Revisione indipendente — barra e cattura schermo

Stato: rilettura indipendente conclusa sullo snapshot sotto riportato.
I due rilievi iniziali sono stati corretti e riletti nei chiamanti; non
restano rilievi concreti aperti nel perimetro esaminato. Questo risultato
non costituisce una garanzia generale di sicurezza o di invisibilità.

## Perimetro e metodo

L'incarico riguarda le finestre proprie di NotchLite: pannello principale,
dashboard, assistente call e orologio. La verifica segue la checklist
Ponytail del progetto, con lettura dei chiamanti, inizializzazione, ricarica
delle impostazioni e distruzione delle finestre. Non comprende il controllo
di finestre di altre applicazioni o l'occultamento del processo dal sistema.

Il revisore non modifica i sorgenti del prodotto e non avvia GUI, nuove
finestre, call, acquisizione audio o condivisione schermo. Gli esiti di test
e compilazione eseguiti dal coordinatore sono registrati separatamente
nella validazione della versione; non sono prove eseguite dal revisore.

## Contratto della piattaforma verificato

- `WS_EX_TOOLWINDOW` esclude la finestra dalla barra e da Alt+Tab.
  `WS_EX_NOACTIVATE` ha invece effetti sul focus: non deve essere richiesto
  soltanto per nascondere un pannello aperto esplicitamente dalla barra.
- `SetWindowDisplayAffinity` riguarda finestre di primo livello del processo
  chiamante. `WDA_EXCLUDEFROMCAPTURE` richiede Windows 10 versione 2004 o
  successiva e DWM attivo. Versioni precedenti possono produrre contenuto
  vuoto invece di escludere completamente la finestra.
- La documentazione di `GetWindowDisplayAffinity` limita il successo alle
  finestre layered. Una rilettura fallita non equivale da sola a un rifiuto
  della precedente richiesta Set; questa distinzione è stata segnalata
  all'implementatore prima del codice.
- La richiesta accettata dall'OS non prova che una specifica applicazione
  di call o modalità di acquisizione sia stata collaudata. Non è una garanzia
  universale di protezione del contenuto.

Fonti Microsoft consultate il 26 novembre 2026:
[stili](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles),
[SetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity),
[GetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowdisplayaffinity).

## Rilievi chiusi dopo la correzione

1. **Applicare la richiesta prima di un nuovo ShowWindow.**
   Nella prima modifica `App::reload_settings` impostava la nuova policy, poi chiamava
   `apply_panel_style`, che può nascondere e mostrare la finestra, e solo
   dopo applicava l'affinità. Se il file cambia contemporaneamente Tray only
   e l'esclusione da disattivata ad attiva, il pannello viene mostrato prima
   della nuova richiesta. Ora `src/shell.rs:296` applica l'affinità prima
   del cambio stile e registra l'errore prima che un chiamante possa
   mostrare il pannello. Caso ricavato dai chiamanti, senza prova GUI.

2. **Lo switch deve invertire il valore mostrato.**
   I due switch privacy leggono il valore attivo dalla policy, mentre
   la prima modifica di `SettingsPage::toggle` invertiva il file appena letto. Dopo una
   modifica esterna non ancora applicata, un click che mostra Off può leggere
   On dal file, scrivere Off e non attivare ciò che l'utente ha richiesto.
   Ora `changed_privacy_setting` (`src/pages/settings.rs:89`) inverte
   il valore attivo mostrato e lo inserisce nella mappa fresca, preservando
   le altre preferenze. La regressione pura in `src/pages/settings.rs:482`
   copre la divergenza tra file e policy e le altre chiavi esterne.

## Riscontri sul sorgente riletto

- Il main nasce nascosto, applica l'affinità prima della prima visibilità e
  ripete la richiesta al reload. Gli errori impediscono Show e redraw del
  pannello, interrompono l'animazione e compaiono nel tooltip/menu tray;
  il menu permette di disattivare l'esclusione per recuperare il pannello.
- Dashboard, call e orologio applicano la policy prima di mostrare nuove
  finestre e prima di riutilizzare quelle esistenti. Un errore chiude le
  finestre ausiliarie coinvolte e disattiva la loro disponibilità.
- `Assistant::sync` revoca l'ascolto all'inizio della ricarica prima delle
  modifiche alle finestre. I percorsi di errore mantengono uno Stop aggiuntivo.
- Alla chiusura da tray `stop_before_exit` chiama `stop_call` prima di
  `DestroyWindow`, mentre App è ancora preso dal gestore del menu. La revoca
  della generazione audio precede la pulizia dei provider. `WM_DESTROY`
  tenta la stessa revoca con `try_borrow_mut` per gli altri percorsi, prima
  della distruzione degli overlay, senza richiedere un secondo prestito
  durante Exit. Riletti anche il teardown e il backstop in `Drop`.
- `panel_style` consente l'attivazione del pannello esplicito anche in Tray only;
  `apply_panel_style` sostituisce solo i tre bit di modalità, conservando
  gli altri stili. La capsula resta senza attivazione come in precedenza.
- Il registro della policy viene ripulito su distruzione. Le guardie degli
  oggetti distrutti evitano di cancellare lo stato di un HWND riciclato.
  Un errore resta segnalato anche quando la finestra fallita viene chiusa.
- I chiamanti usano solo HWND delle finestre proprie. Nessuna modifica alle
  finestre Codex Desktop, browser o ad altri processi è introdotta.
- Lo stato disegnato da Settings è memorizzato, senza nuove letture di file
  o chiamate native nel percorso di disegno. Versione e DWM sono controllati
  prima di richiedere esclusione completa. La rilettura Get distingue
  richiesta accettata da valore effettivamente riletto.
- `--privacy-check` termina prima del normale avvio, legge solo metadati OS
  e dichiara esplicitamente di non aver creato finestre, applicato l'affinità
  su una finestra reale o collaudato una condivisione.

Sono presenti sei nuove regressioni pure: default e valori espliciti,
stili del pannello e focus, versione/DWM, classificazione della rilettura,
switch con file esterno e chiusura logica delle viste su errore. Non chiamano
`SetWindowDisplayAffinity` su HWND reali. L'esecuzione è a cura del coordinatore.

Sul congelamento finale il coordinatore ha comunicato 137 test superati
e build release GNU riuscita in 38,33 secondi. Il revisore ha verificato
nuovamente la piccola correzione al teardown e gli hash; non ha ripetuto
queste esecuzioni. Non sono state usate GUI per queste verifiche.

Non verificati: comportamento visivo della barra/Alt+Tab, input nativi,
copertura effettiva di una condivisione schermo, comportamento del singolo
backend di cattura, prestazioni e RAM del prodotto in GUI.

## Snapshot dei sorgenti esaminati

SHA-256 del 26 novembre 2026. I documenti non fanno parte di questo snapshot;
eventuali cambiamenti ai seguenti sorgenti richiedono una nuova verifica.

| File | SHA-256 |
| --- | --- |
| `src/window_policy.rs` | `338f76cdbead84547c1bb4cfa252881b1eb769bc9aad22ce03895acfdc6ec645` |
| `src/shell.rs` | `4e992dfb1e92e762474ff84127b971b0d424c3bdb0ca22792bfe871504535a1c` |
| `src/pages/settings.rs` | `998ae1becd6a81e685eb1c6b02f7b1411b61a30387f5953f1490994725c9257d` |
| `src/assist.rs` | `634225d05cc1aaffb3eb2092f88614d3c425fbae05213d6b51b923ec8060a57f` |
| `src/assist_overlay.rs` | `04fee1244bfba5a25a4c0487463d3f4da599ed37e25b3106d8471bfcb56ae69e` |
| `src/desktop_clock.rs` | `1bb2aec2f50188daa07af43ca23a37c8626aaae1e3d7728f84324be37dcdb667` |
| `src/main.rs` | `3fdb9a2f6659a778039a160196a05c955d7bff0e2f8382467e35251d53ad7298` |
| `Cargo.toml` | `9e3d3366bbeb39670d1d59408bae89fa569e66852a8e1b1d73597c2264be6637` |
| `Cargo.lock` | `4e5d76b09981d249c532af2def1dcbd6b68f15509fe51f328a600b2fada2f79e` |
