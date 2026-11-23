# Revisione di correttezza, prestazioni e lifecycle

Revisione statica del 23 novembre 2026. Base: Ada Notch `5a1e17f040672a71685872b1b234e01d02a47bb5` più modifiche locali NotchLite. Non è una certificazione di qualità o sicurezza. Nessuna app, PoC, scanner o test del target è stato eseguito da questo revisore. Il coordinatore ha successivamente comunicato 15 test unitari passati e compilazione release riuscita; queste esecuzioni non sono state ripetute dal revisore.

## Esito

Quattro difetti dimostrabili dai sorgenti sono stati corretti in `src/shell.rs`, entro l'incarico assegnato. Le chiusure dell'interfaccia Windows devono ancora essere provate sul runtime. Restano lacune su animazione in fullscreen, DPI e gestione errori di scrittura. Q5, inizialmente segnalato sulle impostazioni, è stato respinto dopo verifica indipendente del refresh al rientro del puntatore.

## Difetti corretti nei sorgenti

| ID | Priorità | Innesco ed effetto prima della modifica | Traccia prima | Correzione e riferimenti dopo |
|---|---|---|---|---|
| Q1 | P1 | Exit dal menu tray distrugge la finestra durante un borrow mutabile APP; il WM_DESTROY reentrante viene ignorato e il message loop può restare attivo senza finestra, mantenendo il mutex e impedendo il riavvio | shell.rs 808–836 → wndproc 1007–1011 → ramo cleanup 1052–1057 non raggiunto | WM_DESTROY gestito prima del borrow, shell.rs 1039–1052; rimozione tray e PostQuitMessage non dipendono dal RefCell |
| Q2 | P2 | Cambiare l'avvio automatico di NotchLite elimina anche la voce startup YESDEV appartenente alla linea originale | shell.rs 918–934, cancellazione legacy a 920–921 | Rimosso il delete legacy; set_startup modifica soltanto RUN_VALUE NotchLite, shell.rs 953 |
| Q3 | P2 | Digitare un carattere fuori BMP, ad esempio un emoji, produce due code unit UTF-16; entrambe vengono scartate da char::from_u32 | shell.rs 1029–1032 | Decoder UTF-16 privato, shell.rs 56–79 e WM_CHAR a 1073; reset a cambio editor/focus. Due test puri coprono BMP, coppie valide, surrogati isolati e reset |
| Q4 | P2 | Cambiare pagina durante una bozza Todo/Calendar cambia il destinatario dei tasti senza concludere l'editor uscente; la bozza resta attiva e non salvata nella pagina nascosta | shell.rs 373–381, 626–640, 720–725; Todo on_focus_lost a 355–360 e Calendar a 505–510 | go_to conclude l'editor prima di cambiare current, shell.rs 400–411; helper finish_page_edit a 750–754, condiviso con end_keyboard |

### Tentativo di confutazione

Q1: il fallback DefWindowProc non esegue il cleanup applicativo né inserisce WM_QUIT. La rientranza è prevista dallo stesso commento di wndproc. [DestroyWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-destroywindow) invia WM_DESTROY e WM_NCDESTROY: il problema dipendeva dall'ordine del borrow, non da un input malevolo.

Q2: RUN_VALUE è già NotchLite; nessun requisito di migrazione dei dati originali giustifica cancellare YESDEV.

Q3: il TextField gestisce correttamente Rust char e UTF-8; l'errore era prima di quel livello. [WM_CHAR](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-char) usa code unit UTF-16 per una classe Unicode. L'assenza di WM_UNICHAR non fornisce un percorso alternativo ai normali messaggi WM_CHAR.

Q4: Mouse::Leave cancella solo hover nelle due pagine. Le frecce/dot ritornano prima di page_mouse e nessun on_focus_lost interveniva al cambio di current. La correzione non chiama il percorso di collasso, per non introdurre una chiusura del pannello durante la navigazione.

### Prove di chiusura richieste

- Q1: selezionare Exit dal tray, verificare fine processo e riavvio immediato. La sola uscita forzata con Stop-Process non prova il ramo corretto.
- Q2: con valore sintetico YESDEV presente in un hive di prova isolato, togglare soltanto NotchLite e verificare che YESDEV resti invariato. Non usare valori reali dell'utente come fixture.
- Q3: i test del decoder sono inclusi nella suite passata comunicata dal coordinatore; resta la prova di testo misto nell'interfaccia.
- Q4: aggiungere una bozza Todo, passare a Focus, tornare a Todo e verificare un solo salvataggio e nessun tasto indirizzato alla bozza nascosta.

## Residui aperti

### Q6 — L'animazione media resta pianificata anche quando la notch è nascosta dal fullscreen

Priorità P2 per l'obiettivo di consumo minimo. `guard_z_order` cambia solo visibilità; `activity_animating`, `start_animation`, `step_animation` e `redraw` non sospendono il lavoro quando hidden_for_fullscreen è true. Con musica attiva, l'attività continua a richiedere frame. Il fatto che i messaggi e il rendering continuino è supportato dal codice; la quantità di CPU/GPU spesa non è stata misurata e non si attribuisce una percentuale.

Riferimenti: shell.rs funzioni activity_animating, guard_z_order, step_animation e redraw; media.rs activity_animating. Proposta: fermare il timer di animazione e omettere la presentazione mentre nascosta, mantenendo timer funzionali e stato; riprendere/ridisegnare al ritorno visibile. Misurare in entrambi gli stati.

### Q7 — La scala DPI viene fissata all'avvio

Lacuna dimostrata, effetto visivo da verificare. L'app dichiara consapevolezza per-monitor in main.rs, ma shell inizializza scale con GetDpiForSystem soltanto all'avvio (post-patch shell.rs 1125), crea renderer e swapchain una volta e non gestisce WM_DPICHANGED/WM_DISPLAYCHANGE. Cambiando scala o configurazione monitor il codice non aggiorna dimensioni, posizione e fattore di conversione input.

[Microsoft WM_DPICHANGED](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged) richiede aggiornare risorse e dimensione/posizione della finestra per le app per-monitor aware. Proposta: implementare un percorso unico di resize/DPI con renderer aggiornabile; provare 100/150/200%, cambio monitor principale e scollegamento. Non è stato osservato un errore sul display dell'utente.

### Q8 — Errori di salvataggio ignorati

Lacuna dimostrata, innesco di errore non riprodotto. Todo save (todo.rs 93–97), Calendar save (calendar.rs 213–217), Focus save (focus.rs 174–179), Settings toggle (settings.rs 154–156) e shell set_setting scartano gli errori di store::write_*. Con disco pieno/permessi insufficienti l'interfaccia può mostrare una modifica riuscita che non sopravvive al riavvio.

Proposta: propagare l'errore e mostrare un esito minimo di salvataggio, preservando l'ultimo contenuto valido. La simulazione va eseguita su directory sintetiche; non cambiare permessi o spazio disco reali dell'utente.

### Q9 — La prima configurazione non viene completata atomicamente

Lacuna dimostrata, interruzione non riprodotta. profile.rs 11–16 crea direttamente il file finale; un fallimento di write_all lascia un file parziale. Al riavvio AlreadyExists viene accettato senza validazione e la lista disabled mancante/monca può far usare i default originali più ampi.

Proposta: inizializzazione atomica e politica esplicita per il recupero di un profilo incompleto, senza sovrascrivere impostazioni già valide. I test dovrebbero coprire un errore dopo la creazione del file, oltre al caso di un file esistente sano già coperto.

## Verifiche incrociate e ipotesi respinte

- Q5 — ipotesi di Settings obsoleto dopo toggle dal tray: respinta. La prima analisi aveva considerato soltanto costruttore/toggle e omesso la guardia in `SettingsPage::on_mouse`. `Mouse::Leave` resetta inside (settings.rs 222–224); al primo `Mouse::Move` successivo la pagina rilegge settings.txt e aggiorna self.map (205–213). Il normale giro puntatore → tray → Settings attraversa questi eventi. Non è stato trovato un percorso utente concreto che eviti la guardia e produca l'effetto segnalato. Nessuna modifica dei sorgenti è stata eseguita per questa ipotesi, nessuna priorità di difetto confermata le è attribuita.
- URL della shelf: confermata indipendentemente la validazione diseguale tra UniformResourceLocatorW e testo normale nel codice pre-correzione. Si veda il report sicurezza. È dimostrata l'ammissione/scrittura di schemi non http(s); non sono dimostrati RCE o assenza di avvisi Windows.
- Worker meteo dopo disattivazione: confermato indipendentemente il percorso pre-correzione. La pagina disabilitata viene lasciata in existing e distrutta da pages::build, ma il worker dormiente ricominciava fetch_weather prima di verificare che il Receiver fosse sparito. Correzione gestita dal revisore sicurezza.
- Ipotesi di cache testi musica senza limite: respinta. media.rs 444–447 svuota la cache oltre 64 voci. Il limite è per numero di voci, non prova un budget in byte.
- Ipotesi di worker musica permanentemente attivo dopo disattivazione: non confermata. MediaPage ha Drop con shared.closed, il ticker lo verifica e il worker rimuove le subscription. Una chiamata WinRT/rete già bloccata può comunque ritardare l'arresto: richiede validazione runtime.
- Ipotesi di bitmap dinamiche illimitate: respinta. gfx.rs 573–587 impone un massimo per numero di immagini; i byte per immagine restano oggetto della revisione input/immagini.

## Copertura

| Unità | Stato | Contenuto verificato | Limite |
|---|---|---|---|
| main.rs, profile.rs, anim.rs | ispezionata | avvio, identità, primo profilo, spring | nessuna prova processo da questo revisore |
| shell.rs | ispezionata | finestre, dispatch, rientranza, timer, input, focus, pagine, tray, startup | interazioni native e shutdown da provare |
| gfx.rs | parzialmente ispezionata | creazione device, frame, cache font/immagini, decoding | nessun reset GPU, stress o consumo GPU misurato |
| pages/mod.rs | ispezionata | lifecycle abilitazione/disabilitazione, riuso pagine | nessun test runtime di tutte le combinazioni |
| focus.rs, todo.rs, calendar.rs, settings.rs, textfield.rs | ispezionata | stato, input, salvataggio, date e timer | nessuna validazione visuale/layout |
| media.rs | parzialmente ispezionata | worker, cancellazione, cache e redraw | integrazione sessioni/lyrics non provata |
| weather.rs, shelf.rs, droptarget.rs | parzialmente ispezionata | verifiche incrociate mirate | audit sicurezza principale svolto da altro revisore |
| notifications.rs, football.rs, system.rs, claude.rs, mascot.rs | parzialmente ispezionata o fuori perimetro assegnato | solo riferimenti e lifecycle pertinenti | nessuna pretesa di copertura completa |

## Snapshot della patch di questo revisore

Unico sorgente modificato da questo revisore: src/shell.rs. SHA-256 dopo la patch: `5f53a38048852777327bb27fbab4e50089511d6bb77e2907bd95fddccf2b8c70`. Modifiche di altri revisori su file distinti sono gestite dal coordinatore. Non è stata alterata la licenza né pubblicato alcun repository da questo agente.
