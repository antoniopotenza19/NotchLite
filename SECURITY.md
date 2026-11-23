# Sicurezza

NotchLite è una preview sperimentale. La revisione documentata riguarda i sorgenti e casi di regressione sintetici; non equivale a una certificazione del prodotto.

Se trovi un problema di sicurezza, usa la segnalazione privata di GitHub se disponibile per il repository. Non pubblicare credenziali, dati personali o dettagli operativi sensibili nelle issue. I report già presenti non dimostrano esecuzione remota di codice.

## Confini attuali

I link creati tramite trascinamento accettano solo HTTP/HTTPS. L'apertura di file scelti dall'utente usa le associazioni di Windows. Disattivare il meteo interrompe l'attesa del worker e impedisce ulteriori refresh; una richiesta già avviata può completarsi.

Restano da approfondire limiti di risposta HTTP e immagini, timeout, codec nativi e uso prolungato. Non sono state eseguite prove di exploit o contro endpoint live. Le superfici coperte e quelle parziali sono nel ledger docs/review/security/coverage.json.
