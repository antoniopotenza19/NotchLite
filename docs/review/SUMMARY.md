# Revisione prima della pubblicazione

23 novembre 2026. Tre agenti separati hanno esaminato sicurezza, correttezza/lifecycle e architettura. Un secondo passaggio indipendente ha riletto le patch con la checklist di Ponytail. Il grafo è stato estratto localmente con Graphify già installato.

## Correzioni

| ID | Caso concreto | Correzione |
|---|---|---|
| Q1 | Exit distrugge la finestra sotto un borrow rientrante e può lasciare il processo attivo | Cleanup WM_DESTROY prima del borrow |
| Q2 | Cambiare avvio NotchLite cancella una voce startup legacy di un'altra app | Rimossa la cancellazione estranea |
| Q3 | WM_CHAR perde caratteri UTF-16 fuori BMP | Decoder delle coppie surrogate |
| Q4 | Cambiare pagina lascia l'editor precedente attivo | Conclusione editing prima della navigazione |
| S1 | Formato URL OLE e writer accettano schemi che il testo normale rifiuta | Validazione HTTP/HTTPS unica prima del salvataggio |
| S2 | Il worker meteo dormiente effettua ancora un refresh dopo disattivazione | Stop channel e attesa interrompibile |

Queste sono sei correzioni di correttezza e validazione: non sono sei vulnerabilità critiche e non dimostrano RCE.

## Verifica

La suite normale passa 15 test su 15 con dati sintetici, callback sostitutive per fetch e directory temporanee del progetto. Non vengono aperti handler URL né chiamati endpoint reali dai test aggiunti. Le prove di exploit restano statiche perché manca un sandbox OS adeguato. L'avvio GUI e la RAM misurati nella prima build non sostituiscono un collaudo interattivo della build corretta.

## Residui e copertura

Restano da affrontare DPI/monitor multipli, animazioni durante fullscreen, errori di persistenza, inizializzazione atomica del profilo e budget HTTP/immagini. L'ipotesi sullo stato obsoleto di Settings dopo il tray è stata ridimensionata dal verificatore: il normale ingresso del mouse aggiorna già la pagina. Questi punti sono registrati nei report di dettaglio; la preview non dichiara parità completa con NotchPixel.

La codebase riutilizza prevalentemente Ada. Il lavoro corrente è una derivazione con correzioni e un profilo diverso, non una riscrittura integrale.
