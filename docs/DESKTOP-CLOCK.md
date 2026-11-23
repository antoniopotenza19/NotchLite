# Orologio indipendente — preview 0.1.4

Questo modulo è il primo elemento separato dalla notch verso il desktop coordinato. È disattivato per impostazione predefinita: la sola presenza del codice o l'aggiornamento della preview non lo attiva.

## Comportamento previsto

Settings > **Desktop clock** salva l'attivazione. Il widget mostra ora in formato 24 ore e data locale, con il preset della notch, incluso Rose dalla 0.1.5. Il cambio preset riutilizza la finestra esistente. Per disattivarlo si usa lo stesso interruttore.

La prima versione usa una posizione fissa nell'area di lavoro del monitor principale, in alto al centro sotto la notch. Non introduce ancora trascinamento o posizioni personalizzate.

La visibilità è limitata al desktop attivo: la finestra viene mostrata quando la finestra in primo piano ha classe Progman o WorkerW, e nascosta quando un'altra applicazione è attiva. È una politica intenzionalmente limitata: non equivale a un widget ancorato dietro le icone del desktop e non garantisce visibilità nelle parti scoperte del desktop mentre un'app è attiva. Riconoscimento della shell, Win+D, Peek, desktop virtuali, focus e cambi monitor richiedono ancora prove reali.

La finestra non è topmost, non richiede attivazione e lascia passare i clic. Non modifica wallpaper, taskbar o finestre di Explorer e non installa hook. La [documentazione Microsoft sulle finestre layered](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows) descrive il passaggio dei clic con WS_EX_TRANSPARENT; il comportamento effettivo di questo modulo resta da collaudare.

Il controllo della visibilità usa il tick già presente, quindi può seguire un cambio di primo piano con circa un secondo di ritardo. Se l'avvio o l'aggiornamento del modulo falliscono, la notch segnala l'indisponibilità; per ritentare si può disattivare e riattivare Desktop clock in Settings.

## Costo e ciclo di vita

Il modulo usa GDI sul thread UI esistente. Non crea un secondo dispositivo Direct3D, motore web, thread o timer. Usa il tick già presente e aggiorna il contenuto quando cambiano minuto, data, visibilità, scala o preset. Con il modulo spento non viene creata la finestra né i suoi font.

La procedura messaggi è separata da quella della notch; la chiusura del solo widget non deve terminare l'app. L'uscita generale chiude anche la finestra e rilascia le sue risorse. Un errore del widget viene segnalato nella notch senza impedirne l'uso.

Queste sono caratteristiche del codice, non una misura di RAM o GPU. Il costo aggiuntivo dell'orologio acceso deve essere misurato in una successiva sessione autorizzata alla GUI.

## Verifica e limiti

Per richiesta dell'utente questa iterazione usa soltanto codice, test sintetici e compilazione da riga di comando. L'app e il widget non vengono avviati; nessun desktop viene usato o modificato.

Le prove CLI coprono flag disattivato/attivazione esplicita, conservazione delle altre preferenze, formattazione dell'ora, cambio del giorno e calcolo dei limiti della finestra. Resa grafica, risorse effettive dopo aperture ripetute, DPI, fullscreen e comportamento sopra le icone restano da verificare.

Sfondo coordinato, dock, vetro e altri widget restano nella [roadmap del desktop](https://github.com/antoniopotenza19/NotchLite/blob/main/docs/DESKTOP-EXPERIENCE.md).

Risultati di questa iterazione: [34 test e build](https://github.com/antoniopotenza19/NotchLite/blob/main/docs/review/iteration-4/VALIDATION.json), [revisione indipendente](https://github.com/antoniopotenza19/NotchLite/blob/main/docs/review/iteration-4/REVIEW.md). Il test di chiusura differita usa un manager vuoto senza creare finestre o font.
