# Revisione indipendente — orologio desktop 0.1.4

Data: 2026-11-23. Revisore: agente security_audit, separato dagli autori delle modifiche. Metodo: lettura dei sorgenti finali e dei chiamanti secondo AGENTS.md e Ponytail Review; verifica delle ipotesi contro i percorsi effettivi del codice e la documentazione Win32. Nessuna modifica al prodotto da parte del revisore in questa iterazione.

**Esito:** nessun ulteriore difetto bloccante individuato nel perimetro statico descritto. Il problema di identità dell'handle è stato corretto. La chiusura durante un prestito del manager è stata irrobustita e coperta da un test sintetico. Questo esito consente di valutare la preview come codice compilato e revisionato; non certifica il comportamento grafico, le risorse effettive o la sicurezza dell'intero prodotto.

## Perimetro e copertura

| Area | Evidenza letta e conclusione statica |
|---|---|
| Attivazione | `src/desktop_clock.rs:29`, `src/profile.rs:15`, `src/pages/settings.rs:104`: abilita solo il valore esatto `1`; chiave assente e primo avvio sono spenti. Il controllo è separato dalla lista delle pagine abilitate. |
| Configurazione e isolamento | `src/shell.rs:284`: lettura fallibile prima delle mutazioni, applicazione palette, `sync` opzionale. Un errore dell'orologio viene segnalato senza interrompere l'applicazione delle altre preferenze. Il solo cambio dell'orologio non cambia la guardia di ricostruzione delle pagine. |
| Temporizzazione | `src/shell.rs:820` riusa il campione di ora locale del tick esistente. Il modulo non introduce timer, thread, processo, renderer o dispositivo Direct3D. Con manager vuoto il suo tick ritorna senza creare finestra/font. |
| Stato Win32 | `src/desktop_clock.rs:230`, `:241`, `:419`: il Box conserva un indirizzo stabile durante creazione, callback e distruzione. USERDATA è un puntatore preso in prestito; nessun `Box::from_raw` o rilascio concorrente della stessa allocazione. La distruzione della finestra precede il rilascio del Box e dei font. |
| Font e regione | `src/desktop_clock.rs:160`, `:183`, `:298`, `:315`: il fallimento del secondo font libera il primo; la sostituzione DPI rilascia i vecchi font fuori dal prestito del paint. La regione viene liberata sul fallimento di SetWindowRgn; sul successo passa a Windows. |
| Disegno | `src/desktop_clock.rs:384`, `:442`: BeginPaint/EndPaint sono accoppiati; il font precedente, colore testo, modo sfondo e colore DC_BRUSH sono ripristinati. La stock brush non viene cancellata. I testi hanno buffer delimitati e il disegno usa i token della palette corrente, senza letture di file né creazioni di font nel paint. |
| Errori e uscita | `src/desktop_clock.rs:52`, `:68`; `src/shell.rs:1165`, `:1359`: un errore del tick rimuove il widget e impedisce la stessa segnalazione a ogni secondo. La chiusura principale è fuori dal prestito di APP e la chiusura del widget non pubblica WM_QUIT. Il manager rimuove la finestra prima del relativo Drop. |
| Documentazione e dipendenze | `docs/DESKTOP-CLOCK.md` distingue intenzione e prova reale; `Cargo.toml` estende le API della dipendenza Windows già usata, senza nuova libreria. La classe della finestra rimane registrata nel processo dopo la disattivazione; non viene promessa assenza assoluta di ogni costo del modulo. |

## D1 — identità di HWND dopo una distruzione separata

**Stato: confermato nel sorgente precedente, corretto nel sorgente finale.**

Condizione concreta: WM_CLOSE del solo widget raggiunge DefWindowProc, che distrugge la finestra; il manager conserva ancora il valore HWND. Un successivo IsWindow può risultare vero se Windows ha riutilizzato quel valore per una nuova finestra. Nel percorso precedente le operazioni successive di layout/visibilità potevano quindi riferirsi a una finestra diversa. [WM_CLOSE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close) e [IsWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindow) documentano rispettivamente la distruzione predefinita e il riuso degli handle.

Tentativo di confutazione: IsWindow controlla validità, non provenienza; azzerare soltanto USERDATA non modifica il valore conservato in ClockWindow. Non è invece dimostrata una distruzione di finestre di altri processi, né una riproduzione reale del riuso dell'handle: questi effetti non vengono attribuiti al finding.

Correzione verificata: `WindowState.destroyed` (`src/desktop_clock.rs:199`) viene impostato in WM_NCDESTROY prima di azzerare USERDATA (`:434`). `tick` (`:330`), `invalidate` (`:280`) e Drop (`:365`) verificano tale stato prima di riusare l'handle conservato. Il Box resta proprietà del manager fino al rilascio normale. Il controllo chiude il percorso tra una distruzione già avvenuta e il successivo tick/reload/Drop. Non è stato eseguito un test nativo di distruzione/riuso sul desktop.

## H1 — chiusura rientrante del manager

**Stato: hardening applicato; nessun crash Alt+F4 ordinario dimostrato.**

La versione iniziale teneva un RefMut durante alcune chiamate User32 e `shutdown` richiedeva a sua volta `borrow_mut`. Se una callback richiedeva la chiusura in quell'intervallo, il secondo prestito poteva andare in panic. Il contesto delle chiamate Win32 richiede attenzione ai messaggi sincroni e alla rientranza, come descritto da [Microsoft sui messaggi e le code](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-messages-and-message-queues).

Tentativo di confutazione: il WndProc del widget non accede al manager; inoltre una normale pressione Alt+F4 è inizialmente accodata. Non è stato stabilito un percorso completo che, senza ulteriore pumping/callback, trasformi quella pressione in una distruzione della notch durante una specifica chiamata User32 del widget. Il finding non viene presentato come exploit o crash osservato.

Invariante ora verificata sul sorgente: `shutdown` imposta SHUTDOWN_PENDING e usa `try_borrow_mut` (`src/desktop_clock.rs:68`). Se il prestito è occupato lascia pendente la richiesta. I wrapper `sync` e `tick` drenano dopo il rilascio del prestito, sia in successo sia in errore (`:34`, `:52`). Il drain estrae l'Option, rilascia RefMut e solo dopo esegue Drop (`:74`); non dipende dal recapitare messaggi a una finestra principale già distrutta. Chiusure ripetute con manager vuoto restano innocue. Il test `:472` esercita direttamente prestito occupato, scarico successivo e idempotenza su un thread isolato con manager vuoto; non crea finestre o font e non riproduce un incidente Win32.

## Verifiche e limiti

Il revisore ha effettuato solo lettura dei sorgenti e confronto degli hash. Non ha eseguito build, scanner, target, PoC o test nativi. L'agente principale riporta **34 test passati, zero falliti**, e build release riuscita con `cargo test --locked --offline` e `cargo build --release --locked --offline`; il riepilogo e gli identificatori degli artefatti sono in `VALIDATION.json`. I sei test del clock usano dati puri oppure il manager vuoto; non dimostrano il ciclo di vita di un oggetto GDI reale.

La politica Progman/WorkerW è un controllo semplice della classe in primo piano, non un collegamento persistente al desktop di Explorer. Non sono stati verificati visibilità reale, click-through, Win+D, Peek, desktop virtuali, fullscreen, compositing, DPI/cambi monitor, localizzazione della data, contatori GDI dopo toggle ripetuti, RAM/GPU o latenza. La finestra è configurata senza TOPMOST e senza richiesta di attivazione; il comportamento risultante resta da collaudare in una sessione GUI autorizzata. Non si ereditano misure di prestazioni da versioni precedenti.

## Identificazione del sorgente finale

Hash SHA-256 dei byte locali effettivamente riletti; coincidono con il record dell'agente principale. Una conversione di fine riga può cambiarli senza alterare il contenuto logico.

| File | SHA-256 |
|---|---|
| `src/desktop_clock.rs` | `0b99e89fc0fd7382b1669c35e91a13df4ac60ab2fee2b8e16f9fcb4c58dc91f7` |
| `src/shell.rs` | `f096841d6b252dd7fb4e168c1cfe1cde58a8a6cc9e3e0482dd4a3ea368430040` |
| `src/profile.rs` | `72d27d7a567dc0dc1115bdc765295ead34c08d22a0e4ad5ab3edbb26d83cec32` |
| `src/pages/settings.rs` | `d5f5826eabbf89a6ed5435bb9216189f5dd6fe47758fa155199677faf243b6f1` |
| `Cargo.toml` | `ecaec3feca9bff3d77ca51f7fc192708e6815b49ea012648ca2e7093453a619b` |
