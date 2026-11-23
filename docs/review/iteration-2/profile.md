# Iterazione 2 — Q9, inizializzazione del profilo

## Perimetro

Modificato soltanto src/profile.rs. store.rs, lettura successiva delle impostazioni e logica dei moduli non sono stati riscritti. Revisione circoscritta, senza un nuovo audit totale, scanner o riproduzioni di sicurezza sul sistema dell'utente.

## Problema corretto

La versione precedente apriva direttamente settings.txt con create_new e poi vi scriveva il profilo. Un errore durante write_all poteva lasciare il nome finale associato a byte parziali. Il riavvio accettava AlreadyExists e poteva applicare i default originali per le chiavi mancanti.

Ora il percorso finale non viene usato per la scrittura iniziale:

1. Se esiste già un file regolare, viene preservato senza modificarne neppure un byte.
2. Viene riservato un nome temporaneo univoco nella stessa directory, con create_new.
3. Il profilo viene scritto interamente e viene richiesto sync_all; gli errori si propagano.
4. L'handle viene chiuso.
5. MoveFileExW pubblica il file già completo. È impostato soltanto WRITE_THROUGH: REPLACE_EXISTING e COPY_ALLOWED sono assenti.
6. Se un altro autore crea il file finale nel frattempo, il suo file prevale. Un altro errore di pubblicazione termina l'inizializzazione; non si ripiega su una copia o su una scrittura diretta.
7. Il guard rimuove, per quanto possibile, solo il temporaneo riservato da questa operazione.

Una directory o un link simbolico al percorso finale non viene accettato come configurazione leggibile: l'inizializzazione fallisce chiusa. I percorsi con NUL sono rifiutati prima dell'API di pubblicazione.

## Primitive e portabilità

Non si usano hardlink, quindi la correzione non richiede quella funzione di NTFS e non la presume disponibile su exFAT. Il temporaneo e il finale hanno lo stesso parent; non è autorizzato un fallback di spostamento tra volumi. La riuscita richiede un filesystem che supporti la rinomina richiesta e permessi adeguati. Se non la supporta, il programma propaga l'errore.

Fonti primarie consultate: [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) per opzioni di sostituzione/copia e [File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all) per gli errori di sincronizzazione. Questa soluzione evita la pubblicazione di una scrittura parziale; non costituisce una garanzia transazionale contro ogni guasto hardware o perdita di alimentazione.

## Regressioni aggiunte

Quattro test di comportamento nel modulo, incluso il caso preesistente rinforzato:

- Primo avvio completo; scelte successive preservate senza invocare nuovamente il writer.
- Errore sintetico dopo aver scritto parte del contenuto: nessun file finale, temporaneo eliminato e avvio successivo capace di riprovare.
- File utente creato durante lo staging: nessuna sovrascrittura e nessun temporaneo residuo.
- Directory al posto del file finale: errore, nessuna configurazione parziale creata.

I test usano esclusivamente directory temporanee sintetiche; non modificano ACL, disco disponibile o impostazioni reali dell'utente. Non sono stati eseguiti da questo agente; la suite normale è assegnata al coordinatore. Non è stata eseguita una prova di crash del processo, filesystem exFAT o spegnimento.

## Limite retroattivo

Un file LEGACY già parziale, lasciato da una versione precedente, non viene riconosciuto o ricostruito automaticamente. La semplice assenza di disabled non prova una corruzione: può rappresentare la scelta valida di abilitare tutti i moduli. Modificare questi file sulla base di tale ipotesi rischierebbe di sovrascrivere preferenze reali. La correzione impedisce la nuova finestra di scrittura parziale, ma non promette recupero dei vecchi file ambigui. Un eventuale formato versionato/validato e una procedura di recupero richiedono una scelta ulteriore esplicita.

I temporanei di un processo arrestato senza unwinding possono rimanere nella directory; non vengono letti come settings.txt né riutilizzati troncandoli.
