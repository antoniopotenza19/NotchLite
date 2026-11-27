# Grafite nativa, notch e pannello call

Brief: rendere più moderna la notch e l'assistente call, conservando il
rendering nativo leggero. L'utente deve poter ingrandire il pannello call,
con scelta persistente e senza interrompere l'ascolto durante il resize.

| Ruolo | Graphite |
| --- | --- |
| Fondo opaco | `#14171B` |
| Superficie | `#20242B` |
| Hover / superficie rialzata | `#292E36` |
| Bordo decorativo | `#3B4654` |
| Testo | `#F0F3F6` |
| Selezione e focus | `#ACC6DA` |

Segoe UI Variable Text e icone Fluent già incorporate per la notch;
Segoe UI nativa per gli editor e pulsanti Windows del pannello. Titolo,
testo di lettura, stato e azioni hanno pesi e spazi distinti. Il bordo tenue
serve a separare le superfici; selezione, focus e Stop hanno segnali più forti.

```text
Notch:  [batteria  ora  data]             [icone delle sezioni]
        ---------------- contenuto della pagina -------------
                         nome sezione

Call:   Assistente call                  [Ingrandisci] [Chiudi]
        stato ascolto / sorgente / stato AI
        [app della call]  [obiettivo facoltativo]
        Trascrizione
        [testo scorrevole e selezionabile                    ]
        Suggerimento AI
        [risposta scorrevole e selezionabile                 ]
        [Trascrivi] [Trascrivi + AI]    [Ignora] [Stop]
        indicazione del bordo da trascinare
```

La gerarchia parte dai contenuti reali della call e dalle sue azioni.
Il colore d'accento distingue la scelta corrente; il rosso resta riservato
all'ascolto e a Stop. Spazi e bordi sostituiscono decorazioni animate.
Le altre palette conservano i loro colori e il preset rimane selezionabile.

Misura standard 420×720 DIP, ampia 640×900 DIP, sempre limitata all'area
disponibile sotto la notch. Con dashboard aperta rimangono almeno 320 DIP
per la chat. Preferenza salvata separatamente dalle impostazioni dei moduli;
nessun salvataggio nel disegno o a ogni evento WM_SIZE.

La verifica distingue contrasto e geometria calcolati, prove native e resa
visiva: una schematizzazione offline non equivale a una schermata dell'app.
