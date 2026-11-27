# Revisione indipendente — Graphite e pannello call ridimensionabile

Stato: revisione indipendente conclusa sul freeze finale. I punti di codice
emersi nella lettura sono stati corretti; non restano difetti concreti
aperti nel perimetro esaminato.

Verdetto: adatto alla preview sperimentale con i limiti di verifica sotto
riportati. La revisione del codice non certifica il gesto o la resa visiva.

## Perimetro e metodo

La modifica riguarda palette Graphite, contorno e navigazione della notch,
riuso della geometria Direct2D e aspetto e dimensioni della finestra call.
La lettura segue Ponytail Review e la checklist Emil, con controllo dei
chiamanti, dei percorsi di errore e dei test. Il carico atteso è un utente,
una notch e due pannelli nativi, anche su monitor verticale a origine negativa.

Il revisore modifica soltanto questo rapporto. Non esegue desktop, GUI,
catture o audio e non rivaluta il backend AI invariato. Una verifica di
contrasto numerico o un test di rettangoli non dimostrano la leggibilità,
il focus o il funzionamento di un gesto sulla UI reale.

## Riscontri di interfaccia

| Before | After | Why |
| --- | --- | --- |
| Pannello call di dimensioni fisse | Dimensioni in DIP salvate separatamente, resize nativo e azione compatto/ampio | Dare spazio al testo senza ricaricare settings o richiedere Stop dal percorso di resize |
| Geometria della notch ricostruita a ogni disegno | Cache limitata all'ultima forma | Evitare una cache crescente e condividere la forma tra riempimento e bordo |
| Pulsante iniziale sempre «Ingrandisci», anche riaprendo la misura ampia | Etichetta aggiornata durante position prima di Show | Il nome dell'azione deve corrispondere a ciò che farà il click |
| Preferenza salvata dall'ultima proposta WM_SIZING | Rettangolo effettivo riletto e riconciliato a fine gesto | Un annullamento del gesto non deve ricomparire al refresh seguente |
| WS_BORDER e soli codici di hit-test nel primo draft | WS_THICKFRAME per la call e WM_NCCALCSIZE con client pieno | Usare il contratto nativo di sizing senza ridurre lo spazio dei controlli |
| Peer HWND controllato soltanto con IsWindow | Registro delle finestre, PID, tipo Dashboard e stato non distrutto | Un handle memorizzato deve ancora identificare la finestra propria prevista |
| Bordo tenue usato per le superfici | Focus dei pulsanti con accento pieno o fondo scuro sulla selezione chiara | Distinguere focus interattivo e contorno decorativo |

La proposta Graphite produce contrasto testo/sfondo 16,14:1,
testo/card 13,98:1, testo/hover 12,26:1 e accent/card 8,78:1.
Il bordo ha 1,87:1 sullo sfondo e 1,62:1 sulla card; il codice letto
lo usa come separazione decorativa. Il focus owner-draw impiega invece
l'accento pieno oppure il fondo scuro sulla superficie d'accento.

## Riscontri del sorgente e correzioni

La geometria Direct2D è conservata in una sola Option per Gfx, con chiave
di cinque coordinate/raggi in bit. Un contenuto aggiornato senza variazioni
di forma riusa il path; durante una variazione viene sostituito l'unico
elemento. Il bordo Graphite usa lo stesso path del riempimento. Non sono
stati aggiunti timer, blur o una nuova cache crescente. Questo riscontro
non è una misura del consumo RAM o delle allocazioni totali per frame.

Il resize usa lo stesso layout del controller e aggiorna la dashboard
prima di allargare la call. Condivide la preferenza in memoria con il
refresh, che non riposiziona la call durante il gesto. Il codice finale usa
WS_THICKFRAME per la call e tratta entrambe le forme di WM_NCCALCSIZE
con client pieno. La geometria minima viene verificata sia piena sia con
un margine prudente di due pixel. La dashboard conserva WS_BORDER.
WM_SIZE ridispone i controlli, WM_SIZING corregge rettangoli, mentre il
salvataggio avviene soltanto a fine gesto o sul pulsante esplicito. Load
è limitato alla prima call necessaria; il JSON ha versione e limiti di
dimensione. Non viene inviato ReloadSettings dal percorso di resize.

Sono stati corretti dopo il riscontro indipendente l'etichetta iniziale
incoerente con la misura salvata e la riconciliazione del rettangolo finale.
È stato rimosso un HBRUSH superfluo: il disegno usa il pennello DC stock;
il pennello highlight rimasto ha invece un uso concreto nel focus.

Riferimenti delle correzioni: `assist_overlay.rs:488` aggiorna l'etichetta
prima di Show; `assist_overlay.rs:986` legge la misura finale;
`assist_overlay.rs:400` verifica l'identità del peer e la invalidazione
avviene su NCDESTROY; `assist_overlay.rs:953` conserva il client per il
frame nativo. Il test a riga 1043 rifiuta un handle non registrato senza
interrogare finestre reali. La revisione non riproduce un exploit o un
riciclo reale di handle.

I pulsanti restano controlli BUTTON con WS_TABSTOP, evento BN_CLICKED e
instradamento tramite IsDialogMessage. Le nuove risorse GDI vengono
consegnate ai controlli prima di eliminare le vecchie e il DC è ripristinato
dopo il disegno owner-draw. La geometria minima conserva le azioni nel client;
numero di righe native effettivamente leggibili, gesture e focus osservati
non sono collaudati da questi controlli sul codice.

## Esito e validazione

Non restano difetti concreti aperti nel perimetro riletto. Nel codice sono
stati controllati cache, risorse, focus, vincoli del resize e persistenza;
i percorsi di creazione e ricarica applicano ancora l'esclusione dalle
catture prima della visibilità, con chiusura su errore come nell'iterazione
precedente. Il resize non modifica l'affinità della finestra già protetta.

I test nuovi coprono JSON e DIP, contrasto dei token, rettangoli massimi,
dashboard minima, ancoraggio e peer non registrato. Il coordinatore ha
comunicato 161 test superati, zero fallimenti, build release GNU riuscita
in 31,89 secondi e 51 impronte di input identiche prima e dopo. Il revisore
non ha eseguito cargo e ha ricontrollato i 12 hash sotto riportati.

La mappa AST aggiornata dal coordinatore contiene 49 file, 2128 nodi e
5337 archi. La lettura indipendente ha seguito anche i chiamanti dei flussi
modificati; il grafo non prova il comportamento della UI.

Non verificati: gesto mouse nativo, tastiera/focus osservati, metriche e
resa dei caratteri, screenshot reali, condivisione schermo, consumo RAM e
prestazioni della UI. Il backend AI/STT è fuori dal perimetro e invariato.

Fonti Microsoft consultate per il contratto dei messaggi:
[WM_SIZING](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-sizing),
[WM_GETMINMAXINFO](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-getminmaxinfo),
[WM_EXITSIZEMOVE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-exitsizemove),
[Button styles](https://learn.microsoft.com/en-us/windows/win32/controls/button-styles).
Per i due punti finali:
[Window styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles),
[Custom frame](https://learn.microsoft.com/en-us/windows/win32/dwm/customframe),
[WM_NCCALCSIZE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-nccalcsize),
[IsWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindow).

## Snapshot esaminato

SHA-256 del freeze del 27 novembre 2026; include i chiamanti significativi
letti. Una modifica ai sorgenti seguenti richiede nuova verifica.

| File | SHA-256 |
| --- | --- |
| `Cargo.toml` | `a951060d08e8a1a0bb8b3d4ee6f21fa3115a4eeaf403adbc9587de2d50a7be62` |
| `Cargo.lock` | `426b855782793298bc2067f018cb0a90b6a9a8e9f3778e7f65bc4b32ecf76ca4` |
| `src/main.rs` | `43ed2d88d384b8966d417ac9f2b42423762a0480e6288583d6c52cb8982b04d8` |
| `src/theme.rs` | `d4a939ae6fe0fd6733ef65ce0a94eee5542333ae50a43ea18202007500e3d94b` |
| `src/gfx.rs` | `8a428448fdcf0fcbd0fc28895213ab74f148d802f611b97504c67964fc337ebf` |
| `src/shell.rs` | `724e6eb474656d904dccfdb3728f749633a1ce967406424eb3d01b2f59f4d452` |
| `src/assist_layout.rs` | `01c669950e2a6d3ffc4087954bd995e3081cb2556716f4419e47e530cff97ec9` |
| `src/assist_overlay.rs` | `d452fccc3845326aed1fd8a3c663ac905922e753d689043a8a13777aebe1d828` |
| `src/call_panel_size.rs` | `054cce281eadb0e98e125be5c5c8246f1f830e02738c94a91720c4f6c389e6b2` |
| `src/store.rs` | `a6ee9e04817cdd5b8c9bab89311e0399bca759dcd2e9eb9a53140222d9102f34` |
| `src/window_policy.rs` | `338f76cdbead84547c1bb4cfa252881b1eb769bc9aad22ce03895acfdc6ec645` |
| `src/assist.rs` | `1c3ea634f4172b93146c8fb60c2ecd157bfc14b856d4f9ee74a6fac90f205d0c` |
