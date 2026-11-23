# Preset di aspetto — 0.1.5

Il primo incremento visivo riguarda la notch e le sue pagine native. Non applica sfondi, temi di Windows, dock o widget desktop.

| Preset | Pannello | Testo | Accento |
|---|---|---|---|
| Original | Nero `#000000` | Bianco `#FFFFFF` | Verde originale |
| Graphite | Ardesia `#181D26` | Bianco freddo `#F4F7FC` | Ciano `#6BDEEF` |
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

Per richiesta dell'utente, il prodotto viene verificato soltanto da riga di comando: nessuna apertura dell'app, automazione del desktop o modifica alle impostazioni reali del PC. Resa effettiva, selettore e messaggio di errore richiedono ancora una prova GUI. La RAM della 0.1.5 non è stata misurata; il report della 0.1.2 resta storico e non va attribuito alla nuova build. L'immagine nella documentazione è uno schema offline, non una schermata della build.
