# Esperienza desktop completa

Priorità aggiornata il 23 novembre 2026: l'obiettivo dell'utente comprende il desktop Windows nel suo insieme, oltre alla notch. La preview attuale non realizza ancora questa trasformazione.

## Risultato desiderato

Una composizione coerente di isola superiore, barra inferiore, sfondo, grande orologio e widget. Colori, spaziatura, tipografia e comportamento devono appartenere allo stesso preset. Le descrizioni pubbliche di [NotchPixel](https://notchpixel.com/) e le [note degli aggiornamenti](https://notchpixel.com/updates) confermano taskbar, temi, widget e sfondi animati come funzioni del riferimento; non sono funzioni automaticamente ereditate da Ada.

| Parte | Stato NotchLite | Prossimo risultato osservabile |
|---|---|---|
| Isola/pannello | Presente; 0.1.3 aggiunge tre palette native, verificate solo da CLI | Verifica GUI e coordinamento con il resto del preset |
| Barra inferiore/dock | Da implementare | App aperte/pinnate leggibili; ripristino della barra originale affidabile |
| Orologio e widget sul desktop | Da implementare | Elementi posizionabili che restano sul desktop, senza coprire le app |
| Sfondo coordinato | Da implementare | Preset statico con contrasto adatto a icone e widget |
| Vetro/trasparenze | Da implementare | Effetto opzionale con consumo misurato |
| Sfondo animato | Da implementare | Opzionale; pausa quando il desktop non è visibile e durante giochi |

## Due profili di prodotto

**Leggero:** sfondo statico, composizione pulita, pochi widget e aggiornamenti soltanto quando necessari. Il fatto che sia leggero non richiede un desktop anonimo: il lavoro visivo comprende palette, tipografia, icone e proporzioni.

**Vetro:** stessa composizione, con trasparenze ed effetti selezionabili. Video wallpaper e sfocature continue hanno un budget distinto e non ereditano i risultati di RAM della notch base.

L'utente deve poter vedere l'anteprima, applicare un preset e ripristinare l'aspetto precedente. L'applicazione effettiva di un tema sarà una funzione esplicita dell'interfaccia; questo documento non cambia il desktop del PC.

## Ordine di sviluppo

1. Definire un preset coerente e le impostazioni di aspetto.
2. Aggiungere orologio/widget desktop e sfondo statico coordinato.
3. Realizzare il modulo barra/dock con ripristino e monitor multipli.
4. Aggiungere vetro e animazioni misurandone separatamente il costo.

Questa fase precede telefono, chat streaming e altre integrazioni secondarie. Le funzioni già previste restano nella roadmap.

## Verifica

Confrontare lo stesso PC e la stessa configurazione prima/dopo ogni modulo: memoria privata e working set di tutti i processi coinvolti, memoria GPU quando disponibile, CPU a riposo e durante l'uso, comportamento in fullscreen, sospensione e rimozione monitor. Registrare cosa è attivo. Non promettere che l'intero desktop avrà il costo della sola notch.
