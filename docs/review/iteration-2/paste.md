# Iterazione 2 — Incolla esplicito nei campi

Modificato soltanto src/droptarget.rs per fornire al gestore di tastiera il nuovo helper pubblico `clipboard_text() -> Option<String>`.

Su invocazione legge il data object corrente con OleGetClipboard e riusa il lettore OLE/UTF-16, limitando la copia testuale a 512 code unit. Il lettore storico read_text delega a read_text_limited con usize::MAX: la semantica dei drag non cambia.

GlobalUnlockGuard rilascia il lock globale anche in unwinding; il guard è distrutto prima di Medium, che conserva ReleaseStgMedium. La reference IDataObject viene mantenuta durante la lettura. Errori o formato testuale assente producono None.

Non sono aggiunti history, hook, listener, dipendenze o scritture negli appunti. La chiamata è destinata esclusivamente al Ctrl+V esplicito con un campo in modifica; l'integrazione in shell.rs appartiene al coordinatore. Il campo deve continuare a imporre la propria lunghezza massima e trattare i caratteri di controllo come input non testuale, evitando commit causati da newline incollati.

Verifica di questo agente: ispezione delle firme presenti nei bindings windows 0.58.0 e git diff --check pulito. Nessun accesso o modifica degli appunti globali è stato eseguito da questo agente; compilazione, regressioni normali e prova del campo reale sono lasciate al coordinatore. Il limite di 512 unità riguarda il testo copiato dal lettore, non il costo interno che un provider OLE può sostenere per fornire il data object.
