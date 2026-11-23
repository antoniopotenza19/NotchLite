# Iterazione 2 — sospensione grafica e apertura esplicita

23 novembre 2026. Unico sorgente modificato da questo revisore: `src/shell.rs`. Nessun commit o pubblicazione eseguiti da questo agente.

## Correzione

Quando Windows segnala fullscreen/presentazione e la notch normale viene nascosta (nessuna apertura esplicita del pannello):

- il timer TIMER_ANIM viene cancellato e il suo stato azzerato;
- i percorsi start_animation e set_anim_interval non possono riarmarlo;
- un WM_TIMER già accodato termina prima della simulazione/del rendering;
- redraw termina prima di costruire il frame o chiamare gfx.frame;
- il timer della mascotte opzionale viene sospeso, compresi eventi residui già accodati.

Al ritorno visibile, guard_z_order riabilita il timer della mascotte se richiesto, resetta il suo clock e richiede un frame tramite start_animation. Quest'ultimo resetta anche il clock delle molle perché la sospensione ha impostato animating=false. Le molle possono quindi completare le transizioni sospese senza un passo temporale lungo quanto il fullscreen.

L'inizializzazione verifica adesso il fullscreen prima di reload_settings, che può richiedere il primo frame. La successiva ShowWindow iniziale è condizionata dalla stessa visibilità: avviare l'app in fullscreen non aggira il blocco grafico.

## Stato mantenuto

Il timer funzionale una volta al secondo non viene cancellato. page.tick, poll, comandi, modello musica e timer focus continuano a funzionare. go_to e end_keyboard possono ancora aggiornare stato/editor/target ma attraversano start_animation e redraw protetti. Nessuna cancellazione di worker, IPC, hook tastiera o modalità alternativa di test introdotta.

La rilevazione continua a usare SHQueryUserNotificationState e il tick esistente; i cambiamenti di visibilità dopo l'avvio possono quindi essere rilevati fino al tick successivo. Non si promette arresto istantaneo di ogni attività CPU.

## Apertura esplicita richiesta dall'utente

La nuova API shell è `run(open_panel: bool)`. La scelta del default e il parsing degli argomenti sono responsabilità di main.rs, modificato dal coordinatore: doppio clic apre il pannello, `--background` mantiene l'avvio come capsula; `--open` equivale all'apertura esplicita.

In modalità pannello la finestra reale usa WS_EX_APPWINDOW, senza WS_EX_TOOLWINDOW/WS_EX_NOACTIVATE, per essere ritrovabile in taskbar/Alt-Tab. Non è un mock né una modalità di test: la stessa UI e gli stessi moduli restano attivi. Il puntatore può uscire senza richiuderla. Escape, dopo l'eventuale annullamento di un editor attivo, ripristina la capsula e gli stili originali; torna anche la normale politica fullscreen Q6. Un collasso esplicito dal tray percorre lo stesso ritorno.

Solo mentre il pannello è stato esplicitamente aperto, il fullscreen non lo nasconde. È una scelta dell'utente per poterlo raggiungere; il profilo background conserva la sospensione grafica. La voce startup registrata da shell include adesso il percorso EXE quotato seguito da `--background`.

`request_open_existing() -> Result<bool>` cerca la classe NotchLiteWindow e invia un singolo WM_APP_OPEN senza payload utente; restituisce false se non trova una finestra e propaga il fallimento di PostMessage. Il main del coordinatore lo usa al rilancio di un'istanza già esistente. Una voce tray "Open panel" offre lo stesso percorso senza terminale. Nessun hook globale, provider, rete o dipendenza introdotta.

## Tracce di stato controllate staticamente

Queste sono verifiche dei percorsi reali nel sorgente, non test runtime o un simulatore sostitutivo dell'app:

| Sequenza | Invariante del percorso corretto |
|---|---|
| Frame attivo → fullscreen | hidden=true, animating=false, interval=0, KillTimer(TIMER_ANIM) |
| Worker/tick/editor mentre nascosta | i dati cambiano, start_animation ritorna senza riarmare e redraw senza gfx.frame |
| WM_TIMER già accodato dopo KillTimer | step_animation ritorna prima di step/update_region/redraw |
| Nascosta → visibile | clock grafici ripartono da adesso, viene richiesto un frame anche con dati invariati |
| Avvio durante fullscreen | guard prima del primo redraw e nessuna ShowWindow incondizionata |

È stato verificato l'unico punto SetTimer(TIMER_ANIM) e l'unico punto gfx.frame di shell.rs: entrambi sono dietro le guardie. git diff --check non segnala problemi di whitespace.

## Validazione richiesta

Il coordinatore esegue compilazione e suite esistente. Non sono stati aggiunti test che replicano semplicemente gli if, né costruzioni non valide di App/Gfx per aggirare la dipendenza Windows. La prova significativa della cancellazione timer e della presentazione richiede il runtime nativo:

1. Musica attiva, notch visibile: campionare il riferimento.
2. Portare un'app a schermo intero, attendere almeno un tick: verificare scomparsa e assenza di nuovi frame grafici, mantenendo avanzamento del timer focus.
3. Tornare al desktop: verificare riapparizione e primo contenuto aggiornato senza interazioni aggiuntive.
4. Ripetere partendo direttamente dal fullscreen con `--background` e con mascotte abilitata.
5. Confrontare CPU/GPU e frame presentati nei due stati, separatamente dai consumi già pubblicati sulla build precedente.

6. Aprire normalmente l'eseguibile: verificare pannello, presenza in taskbar/Alt-Tab e permanenza uscendo con il puntatore. Escape deve tornare alla capsula.
7. Con un'istanza background già attiva, rilanciare normalmente: verificare che si apra la stessa istanza e che non si crei un secondo processo persistente.

Nessuna nuova misura di CPU/GPU o prova dell'interfaccia è stata eseguita da questo revisore.

SHA-256 del sorgente dopo sospensione e apertura esplicita: `43f3fdd0a82d6626514ea3891508c23366f031368911f7b9d2356976add67d41`.
