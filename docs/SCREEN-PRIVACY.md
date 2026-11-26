# Barra delle applicazioni e condivisione schermo

NotchLite distingue due impostazioni:

- **Tray only** mantiene il pannello nella modalità finestra di servizio,
  senza pulsante nella barra delle applicazioni e senza voce Alt+Tab.
  L'icona nell'area di notifica permette di riaprire il pannello e uscire.
- **Exclude from capture** richiede a Windows di escludere le finestre
  proprie di NotchLite dalle catture che rispettano questa impostazione.
  La finestra rimane visibile sul monitor locale.

Le due opzioni sono indipendenti. Nascondere il pulsante nella barra delle
applicazioni non cambia ciò che viene acquisito durante una condivisione.
Entrambe sono attive quando manca la rispettiva preferenza. Puoi modificarle
in Settings o dal menu dell'icona della tray. I valori espliciti disattivati
sono conservati.

La ricarica delle impostazioni interrompe una trascrizione in corso prima
di modificare o ricreare le finestre: per riprenderla serve una nuova
conferma. Questo evita di proseguire l'ascolto durante la transizione.

## Finestre e accessibilità

Il pannello principale usa `WS_EX_TOOLWINDOW` quando è attivo Tray only.
Un pannello aperto esplicitamente conserva la possibilità di ricevere focus
e input da tastiera: l'assenza dalla barra non richiede `WS_EX_NOACTIVATE`.
La capsula in background conserva invece la propria modalità senza
attivazione. Dashboard, assistente call e orologio sono già finestre di
servizio separate.

L'esclusione riguarda le finestre di NotchLite, tramite l'API pubblica
`SetWindowDisplayAffinity` con `WDA_EXCLUDEFROMCAPTURE`. Questa API richiede
una finestra di primo livello appartenente al processo chiamante e il
Desktop Window Manager attivo. Il valore di esclusione completa è
supportato da Windows 10 versione 2004, build 19041, e versioni successive;
sulle precedenti Windows può applicare il comportamento di contenuto vuoto.

## Stato e limiti

Un risultato positivo dell'API indica che Windows ha accettato la richiesta,
non che una particolare applicazione di videoconferenza sia stata provata.
`GetWindowDisplayAffinity` può rileggere la configurazione, ma la sua
documentazione limita il successo alle finestre layered con DWM attivo.
L'assenza di una rilettura non costituisce una prova di cattura riuscita o
fallita. La build segnala gli errori del sistema senza dichiarare una
protezione universale.

Se Windows rifiuta l'impostazione richiesta, la finestra principale resta
nascosta e le finestre ausiliarie coinvolte vengono chiuse. L'icona della
tray rimane disponibile: il tooltip segnala il problema e il suo menu
permette di disattivare **Exclude from capture** per usare nuovamente il
pannello senza esclusione. Una nuova applicazione riuscita delle impostazioni
consente di mostrarlo di nuovo. Le configurazioni non supportate non vengono
presentate come esclusione completa riuscita.

Il comando `NotchLite.exe --privacy-check` legge solo versione di Windows e
stato DWM. Non crea finestre, non applica l'affinità a una finestra reale e
non collauda una condivisione schermo; l'output distingue questi limiti.

L'icona della tray e i menu del sistema possono restare visibili perché non
sono fra le finestre cui l'app applica questa esclusione. Questa funzione non
nasconde il processo dal sistema e non cambia le finestre di Codex Desktop,
browser o altre applicazioni. Fotografie del monitor e percorsi di
acquisizione che non rispettano l'API non sono controllati da NotchLite.

Prima di condividere, controlla l'anteprima della tua applicazione di call
nella modalità effettiva che userai: intero schermo o singola finestra.
Durante lo sviluppo non sono state aperte finestre o avviate condivisioni;
il collaudo visivo resta distinto dai test di codice e dalla compilazione.

## Fonti

- [Microsoft: SetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity).
- [Microsoft: GetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowdisplayaffinity).
- [Microsoft: extended window styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles).
