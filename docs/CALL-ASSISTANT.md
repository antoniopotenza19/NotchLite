# Assistente chiamate — modalità solo abbonamento

La scelta corrente dell'utente è usare soltanto il proprio abbonamento Codex. In questa versione **Audio AI è disattivato**: non vengono ascoltati, registrati, trascritti o inviati audio delle chiamate. Il monitor Windows prepara soltanto un elenco di applicazioni con sessioni audio in uscita attive e, quando Windows lo segnala, una possibile chiamata. Il provider vocale non viene simulato.

Il pannello delle chat Codex e i suoi comandi rimangono un'integrazione separata. Un eventuale testo proveniente da una futura trascrizione non deve diventare automaticamente un messaggio, un'approvazione o un comando per le chat.

## Comportamento disponibile

Il monitor è creato soltanto quando il controller abilita la funzione. Usa un worker COM MTA per leggere metadati delle sessioni render e ricevere notifiche. Non apre endpoint microfono, non crea IAudioClient/IAudioCaptureClient e non possiede un percorso di acquisizione PCM.

Una sessione in esecuzione non dimostra che ci sia una chiamata: potrebbe essere musica, un video o altro audio. Il suggerimento di chiamata richiede la corrispondenza esatta dell'identificatore di sessione con una notifica communications/ducking di Windows. Quando la corrispondenza manca l'applicazione rimane soltanto una voce del catalogo, senza riconoscimento automatico della chiamata.

Il rilevamento è limitato. Alcune applicazioni gestiscono il proprio mixer e possono non produrre le notifiche previste. Un processo browser può inoltre rappresentare più schede; non è identificata una singola conversazione o persona.

## API del modulo

| Operazione | Effetto attuale |
|---|---|
| `AudioMonitor::start()` | Avvia soltanto il catalogo e le notifiche di metadati. |
| `snapshot()` | Restituisce candidate, suggerimento, errore e `capture: None`. |
| `confirm(key)` | Restituisce sempre un errore esplicito: Audio AI disattivato in modalità solo abbonamento. Non avvia ascolto. |
| `decline(key)` | Nasconde il suggerimento per quell'identità di sessione. Non autorizza altro. |
| `stop_capture()` | Mantiene lo stato senza cattura. |
| `poll_audio()` | Restituisce sempre un elenco vuoto. |
| Drop del monitor | Segnala Stop, rimuove le registrazioni e rilascia le risorse sul worker MTA. |

`AudioTarget` contiene chiave, nome del processo, PID, tempo di creazione del processo, identificatore di sessione e indicazione del possibile ambito esteso. Un PID riutilizzato, un processo riavviato o una nuova sessione ricevono una nuova chiave. Il catalogo conserva un handle del processo per controllarne la terminazione. Sessioni multiprocesso ambigue vengono escluse preservando l'HRESULT nativo; non si considera il PID iniziale come prova di identità unica.

I tipi `CaptureStatus`, `CapturePhase` e `AudioChunk` conservano il contratto del controller per una futura funzionalità distinta. In questa versione nessun percorso costruisce uno stato di ascolto o produce campioni. Non esistono acquisizioni dormienti che possano essere attivate aggiungendo una chiave o cambiando un'impostazione.

## Limiti di risorse e ciclo di vita

Il catalogo usa al massimo 16 endpoint render, 128 registrazioni di sessione e 128 candidate. La coda delle notifiche ducking contiene al massimo 32 eventi; ogni identificatore viene copiato con un limite di 1024 unità UTF-16. Se gli eventi eccedono la coda, il suggerimento viene azzerato e viene esposto un errore. Non ci sono file di registrazione, modelli locali o connessioni di rete.

I callback non accedono allo stato della UI, non attendono spazio nella coda e non rilasciano l'ultimo riferimento agli oggetti WASAPI. Segnalano un aggiornamento al worker; le registrazioni vengono rimosse fuori dai callback. La lettura delle sessioni viene aggiornata dagli eventi e, come controllo di recupero/hotplug, almeno ogni due secondi. Un breve intervallo di correlazione consente alla notifica ducking di precedere l'avvio dello stream.

Drop attende il worker al massimo circa 900 ms. Una chiamata COM già entrata nel sistema può terminare più tardi; gli Arc posseduti dal worker/callback mantengono validi stato e handle fino alla pulizia. Non viene promessa una scadenza assoluta del rilascio kernel. Questi limiti descrivono il codice, non una misura di RAM o CPU.

## Condizioni di una futura acquisizione, oggi assente

Un futuro incremento richiede una nuova scelta esplicita del provider e consenso visibile per la sessione e l'applicazione selezionate. Dovrà mostrare indicatore e Stop anche durante l'avvio, scartare risultati tardivi, fermarsi al cambio di identità e non riprendere automaticamente dopo una nuova chiamata.

La process-loopback API di Windows cattura l'audio riprodotto da un processo e dai suoi figli: non garantisce isolamento di una scheda browser o dell'interlocutore. Non dovrà esserci fallback automatico all'audio di tutto il sistema o al microfono. Per quella API Microsoft indica build 20348 come minimo nella struttura e nel campione; i comuni Windows 10 22H2 build 19045 sono precedenti. La compatibilità effettiva andrà verificata sul target prima di proporre ascolto.

STT italiano locale, modelli, consumo e latenza non sono stati verificati. Un motore SAPI installato, il servizio Voice Access e le API WinRT di riconoscimento sono tecnologie distinte; non viene promessa una trascrizione italiana gratuita preinstallata. La disponibilità della chat Codex non viene presentata come un entitlement vocale automaticamente disponibile a NotchLite.

## Verifica e fonti

I test del modulo usano identità ed eventi sintetici: catalogo senza ascolto, riuso PID, rifiuto della specifica sessione, mancata correlazione della musica e scadenza degli identificatori sconosciuti. Non avviano COM, cattura, microfono o chiamate reali. Compilazione, rilevamento reale delle applicazioni e misure delle risorse sono evidenze separate.

- [Microsoft: registrazione ducking anche senza un proprio stream](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-registerducknotification).
- [Microsoft: identificatore della sessione communications nella notifica](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiovolumeducknotification-onvolumeducknotification).
- [Microsoft: notifiche di sessione, inizializzazione MTA e GetCount](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-registersessionnotification).
- [Microsoft: regole non bloccanti dei callback WASAPI](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nn-audiopolicy-iaudiosessionevents).
- [Microsoft: PID delle sessioni multiprocesso](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-getprocessid).
- [Microsoft: limitazioni del ducking e mixer gestiti dalle applicazioni](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/detect-and-respond-to-audio-state-changes).
- [Microsoft: process loopback, distinto dall'acquisizione di sistema](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/).
