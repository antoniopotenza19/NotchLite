# Pannello call: Lettura e Controlli

Audit del sorgente precedente, non screenshot: sorgente, contesto, modello,
stato e consenso occupavano circa 270 DIP prima dei testi. Al minimo il
testo disponeva di poco spazio. Gli errori di avvio monitor potevano restare
nella sola dashboard. Il problema di compito è leggere durante la call e
capire perché il testo manca.

Direzione approvata dall'utente: grafite con effetto glass, leggera.
Prodotto: aiuto alla lettura e alla risposta durante una call.
Carattere: calmo, concentrato, discreto.
Materiale: vetro scuro nella cornice, grafite opaca sotto il testo.
Momento principale: la trascrizione e la risposta AI, senza modulo sovrapposto.
Segnale persistente: indicatore ASCOLTO e Stop.

- Lettura predefinita: due schede di testo, stato compatto, dimensione e Stop.
- Controlli: sorgente, contesto, diagnostica e consenso in una vista separata.
- La conferma della cattura riporta alla lettura; nessuna attivazione automatica.
- Testo e posizione di scorrimento sono conservati nel cambio vista.
- Layout puri per 96/120/144/192 DPI e larghezza minima 300 DIP.
- Cornice DWM transient Acrylic documentata: 8 DIP ai lati/fondo, 40 DIP in alto.
  Il client libera quella cornice; il corpo e gli EDIT restano opachi per leggere.
- Fallback grafite con OS precedente a Windows 11 22621, composizione/trasparenza
  disattivata, contrasto elevato, risparmio energetico o errore della richiesta.
- Nessun alpha dell'intera finestra, dipendenza, worker o timer aggiuntivo.

Metodo: Unslop di Damir Sokolovsky, adattamento host CC-BY-4.0,
[fonte originale](https://github.com/heydamir/unslop), e Ponytail del progetto.
La lettura del codice e i test geometrici non provano resa grafica, wrapping
di testi lunghi, tastiera o blur effettivo. I metadati DWM confermano solo
la richiesta accettata. Nessun desktop, screenshot o call usati per il design.

Fonti: [DWM system backdrop](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type),
[estensione della cornice](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmextendframeintoclientarea).
