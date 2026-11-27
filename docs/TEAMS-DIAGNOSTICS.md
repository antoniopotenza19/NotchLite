# Teams: pannello visibile senza trascrizione

Il pannello può comparire usando soltanto metadati: questo non dimostra che
la cattura sia stata confermata o che stia arrivando audio. La 0.1.13 separa
questi passaggi e rende leggibili gli errori anche senza aprire la dashboard.

1. Apri **Controlli** nel pannello call e seleziona la sorgente Teams attiva.
2. Scegli **Trascrivi** oppure **Trascrivi + AI**. La conferma resta esplicita.
3. Il pannello torna alla **Lettura**. L'indicatore ASCOLTO e Stop restano visibili.
4. Se manca il testo, leggi il messaggio nella scheda trascrizione e apri
   Controlli per audio ricevuto, campioni sopra soglia, segmenti ed esiti.

| Messaggio o dato | Significato |
| --- | --- |
| Ascolto spento | La rilevazione non ha avviato una registrazione |
| Nessun audio ricevuto dopo 12 s | Non arrivano pacchetti dal processo selezionato |
| Audio silenzioso dopo 12 s | Arrivano pacchetti silenziosi; può essere una pausa |
| Audio ricevuto, ma sotto soglia | Il gate RMS non ha ancora rilevato campioni abbastanza forti |
| Segmenti > 0, esiti ancora assenti | Il motore locale sta elaborando; errori o timeout sono riportati |
| Esiti vuoti | Il motore ha restituito testo vuoto; controllare sorgente e lingua |
| Sessione non disponibile | Proprietario/identità cambiati: scegliere nuovamente e confermare |

La rilevazione Teams include ora sessioni attive anche se l'app è stata
abilitata dopo l'inizio della chiamata o manca un evento ducking Windows.
Una sessione WebView2 viene associata a Teams solo tramite antenati
verificati e limitati. È un indizio per proporre il consenso, non una prova
di chiamata. Non ogni `msedgewebview2.exe` viene trattato come Teams.

Il proprietario corrente della sessione viene ricontrollato tramite
`GetProcessId`. Un cambio PID, un processo terminato o una sessione ambigua
invalidano il vecchio target. Il nuovo proprietario richiede una nuova
conferma. Non si passa implicitamente a un processo fratello, al root Teams
o all'audio globale del PC.

La diagnostica CLI `notchlite.exe --call-audio-check` osserva per circa 2,2 s
solo metadati e restituisce conteggi. Non apre microfono/cattura, non avvia
Whisper o finestre e non stampa PID, nomi o identificatori di sessione.

**Limite:** la causa della call Teams segnalata dall'utente non è stata
riprodotta. Le correzioni riguardano difetti verificabili nel codice e
la diagnosi; nessuna nuova call è stata ascoltata. Il loopback di processo
è indipendente dall'uscita fisica: cambiare cuffie non è una soluzione
dimostrata. La prossima prova nella call deve verificare il percorso reale.

Fonti: [sample Microsoft process loopback](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/),
[GetProcessId e stream switching](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-getprocessid).
