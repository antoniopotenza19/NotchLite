# Aspetto grafite — 0.1.11

Il primo incremento visivo riguarda la notch e le sue pagine native. Non applica sfondi, temi di Windows, dock o widget desktop.

| Preset | Pannello | Testo | Accento |
|---|---|---|---|
| Original | Nero `#000000` | Bianco `#FFFFFF` | Verde originale |
| Graphite | Grafite opaca `#14171B` | Bianco freddo `#F0F3F6` | Azzurro tenue `#ACC6DA` |
| Midnight | Blu profondo `#11182D` | Bianco lavanda `#F3F1FF` | Lavanda `#BBA6FF` |
| Rose | Prugna scuro `#21181F` | Bianco rosato `#FFF2F7` | Rosa `#F3A4C8` |

La direzione segue l'interfaccia nativa compatta: Segoe UI Variable Text per i testi, icone Fluent già presenti, griglia e dimensioni della notch conservate. Carte e stati hover derivano dallo stesso colore di testo. Le etichette secondarie hanno opacità 82%, gli indizi 68%, per aumentare la leggibilità rispetto alla base. I quattro preset sono scuri; un tema chiaro richiede anche adattare badge, copertine e mascotte.

## Uso

In Settings, la prima riga **Appearance** mostra il preset selezionato. Ogni clic passa al successivo: Original, Graphite, Midnight, Rose e di nuovo Original. Il cambio viene salvato in settings.txt e applicato alle pagine e al pannello senza riavviare i moduli già attivi. La [navigazione compatta](https://github.com/antoniopotenza19/NotchLite/blob/main/docs/MODERN-NAVIGATION.md) è nuova nella 0.1.5.

Il primo avvio su una cartella dati nuova sceglie Graphite. Le configurazioni già presenti sono conservate: se appearance manca o contiene un valore sconosciuto, si usa Original. La sola chiave del preset non modifica la lista dei moduli o le altre preferenze. Un errore nella lettura preliminare o nella scrittura mostra un messaggio e non viene presentato come salvataggio riuscito.

Se la rilettura successiva da parte della shell fallisce, l'app conserva la configurazione attiva e attende una futura rilettura riuscita; non abilita i moduli opzionali. Al primo avvio, un errore di lettura impedisce di proseguire con una configurazione vuota. Un file valido e intenzionalmente vuoto conserva invece le semantiche originali delle chiavi mancanti.

Il selettore mantiene il comportamento mouse della griglia Settings. Navigazione da tastiera e UI Automation della griglia restano da sviluppare.

## Costo e limiti

Quattro palette statiche e un selettore atomico di un byte. Nessuna dipendenza, timer, worker, immagine runtime, passaggio di sfocatura o richiesta di rete aggiunti. La risoluzione dei colori non legge file e non alloca memoria. Le risorse grafiche e la cache dei formati del testo vengono riutilizzate.

Un accento già catturato da una notifica in coda conserva il colore al momento della creazione. Emoji, copertine, badge sportivi e mascotte mantengono i loro colori. Non si tratta di Liquid Glass.

I test verificano lettura/ciclo dei preset, conservazione delle impostazioni, fallimenti di persistenza e contrasto dei token di testo compositati sul pannello e sulla carta hover. Il contrasto dei singoli token non certifica accessibilità, sfumature delle animazioni, immagini o interfaccia completa.

Le verifiche iniziali della 0.1.5 furono soltanto da riga di comando; la sua RAM non fu misurata. Il report della 0.1.2 resta storico. La 0.1.11 distingue test di contrasto/geometria, apertura richiesta della copia installata con lettura dei bounds delle sue finestre e misura UI inattiva, da resa visiva e gesti del mouse ancora da collaudare. Nessuna automazione del desktop o cattura della call è necessaria. L'immagine precedente nella documentazione resta uno schema offline, non una schermata della build.

## Pannello call e misure

La 0.1.11 coordina pulsanti call, editor, etichette e selezione della notch con superfici opache. I pulsanti mantengono nome, focus e comportamento nativi; il focus usa un accento ad alto contrasto. La notch riutilizza una singola geometria corrente per corpo e bordo. Non sono introdotti timer di animazione o worker residenti.

Nel pannello call, **Ingrandisci** propone 640×900 DIP e **Compatto** 420×720 DIP, limitati allo spazio disponibile. Il bordo sinistro, quello inferiore e l’angolo inferiore sinistro permettono misure personalizzate. La finestra resta ancorata in alto a destra sotto la notch; con la dashboard aperta le lascia almeno 320 DIP. Le dimensioni sono conservate in call-panel-size.json e vengono salvate solo a fine gesto o su azione esplicita, senza ricaricare i moduli. Un errore mostra **Non salvato** e permette di riprovare.

Geometria, contrasto e compilazione sono verificati separatamente da resa visiva, focus, trascinamento reale e ridimensionamento durante una call. Il registro corrente è [iteration-12/VALIDATION.json](review/iteration-12/VALIDATION.json).

## Pannello call 0.1.13

Lettura e Controlli separano i testi dal modulo di configurazione. Con Graphite la cornice può usare Acrylic DWM nativo (Windows 11 22621+, composizione e trasparenza attive, senza contrasto elevato o risparmio energetico). Il corpo sotto il testo resta opaco; non viene applicata trasparenza all’intera finestra. Il fallback è grafite. Nessun nuovo timer, worker o dipendenza. Una richiesta DWM accettata non dimostra il blur effettivo: la resa visiva resta da collaudare senza usare il desktop.
