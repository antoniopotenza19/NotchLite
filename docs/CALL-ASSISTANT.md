# Assistente chiamate — preview 0.1.10

Abilita **Settings > Call assistant**. Il pannello in alto a destra sul
secondo monitor propone **Trascrivi**, **Trascrivi + AI**, **Ignora** e
**Stop**. Il rilevamento di una possibile chiamata non avvia l'ascolto.

**Trascrivi** acquisisce l'audio riprodotto dall'app selezionata e lo
trascrive localmente con Whisper. **Trascrivi + AI** aggiunge un ponte
testuale al server Codex locale condiviso, che genera suggerimenti automatici
con l'accesso ChatGPT del CLI ufficiale. La modalità semplice non crea quel
collegamento e non richiede un server Codex.

## Uso

1. Prepara il motore locale con `tools/Setup-CallTranscription.ps1`.
2. Quando viene segnalata una possibile call, verifica l'app selezionata.
   Se manca il rilevamento puoi scegliere manualmente un'app con audio in
   uscita. Il campo obiettivo è facoltativo e serve alla modalità AI.
3. Premi **Trascrivi** oppure **Trascrivi + AI**. L'indicatore rosso resta
   visibile durante avvio, ascolto e arresto; Stop rimane disponibile.
4. Il testo compare per frasi successive, dopo una pausa o un segmento di
   circa sei secondi. Il riconoscimento può commettere errori.
5. **Stop** interrompe l'audio e lascia leggibile il testo. **Close** chiude
   il pannello e interrompe l'ascolto. Una nuova sessione richiede conferma.

**Settings > Notch monitor** sposta la notch principale fra gli schermi.
**Assistant monitor** sceglie separatamente il secondario dei pannelli.
Quando sono sullo stesso schermo, il pannello call lascia libera la fascia
della notch aperta. Titolo, font più grandi e le etichette **Trascrizione** e
**Suggerimento AI** separano le due aree; il testo segue le frasi nuove solo
se non hai scorso manualmente verso l'alto. [Monitor e limiti](MONITORS.md).

Sul PC di sviluppo il runtime locale è già preparato. Su un altro PC serve
il setup opzionale. Abilitare la pagina non conferma l'ascolto.

## Chat AI

Nella modalità AI il pannello mostra preparazione, attesa del testo,
generazione ed eventuali errori. I suggerimenti non richiedono che questa
chat desktop continui a lavorare. Usano richieste testuali sul server Codex
locale condiviso e consumano i limiti dell'abbonamento; i contatori disponibili
sono indicati nello stato. Serve un server ufficiale compatibile già avviato:
[preparazione](CODEX-SHARED-SERVER.md), [protocollo e limiti](CALL-LIVE.md).

L'audio resta nel motore locale. La trascrizione recente e l'obiettivo vengono
inviati al servizio AI soltanto dopo **Trascrivi + AI**. Il collegamento usa
`thread/sample`, con campionamenti temporanei e nessuna esecuzione di strumenti
dal testo della call. Il seed persistente del server contiene solo un messaggio
statico. Non copia credenziali, non aggiunge testo alle chat già aperte e non
usa una chiave API con fatturazione separata. Stop revoca anche le richieste AI.
Il ponte manuale per un agente attivo rimane una funzione distinta, documentata
in CALL-LIVE.md.

Le chat già aperte nel server incorporato di Codex Desktop restano distinte
dalla dashboard su server condiviso descritta in CODEX-DASHBOARD.md. Il
ponte call è un'integrazione testuale separata.

## Ambito e rilevamento

L'acquisizione usa la process-loopback API Windows per il processo scelto e
i suoi figli. Richiede build 20348 o successiva; il PC di sviluppo soddisfa
questo requisito. Non usa il microfono e non ripiega sull'audio di tutto il
sistema. Un browser può includere altre schede o suoni: non è garantito
l'isolamento di una persona.

Il catalogo WASAPI correla sessioni render attive e notifiche communications/
ducking per identità di sessione. La musica da sola non prova una call.
Alcune app gestiscono il proprio mixer e non generano le notifiche previste;
per questo è disponibile la selezione manuale.
Anche notifiche di fine/inizio molto ravvicinate sulla stessa sessione
possono non produrre un nuovo suggerimento: il rilevamento è euristico.

PID, tempo di creazione e sessione devono corrispondere. Perdita della
sessione, riuso del PID, errori audio, cambio monitor/DPI e coda piena
interrompono l'acquisizione. Nessun cambio sorgente o riavvio è automatico.

## Risorse e arresto

Il worker MTA possiede e rilascia le interfacce WASAPI. Callback tardive e
parametri posseduti restano validi; le attivazioni pendenti sono limitate a
due. La coda contiene al massimo 32 blocchi PCM da 100 ms, mono 16 kHz.

Il controller forma segmenti limitati con una breve sovrapposizione ai tagli
forzati e conserva anche le risposte brevi. Deduplica soltanto segmenti
consecutivi effettivamente sovrapposti. Code STT e risultati sono limitati;
eventuali sostituzioni vengono segnalate nel pannello.

Stop revoca prima audio e generazioni, poi pulisce IPC e trascrittore.
Risultati tardivi non vengono inoltrati. Un errore fatale STT ferma l'ascolto.
Il testo recente resta nella memoria del pannello. Whisper usa file WAV/testo
temporanei con ACL proprietario/SYSTEM, normalmente eliminati al termine o
su Stop; un crash può lasciare residui. Vedere
[CALL-TRANSCRIPTION.md](CALL-TRANSCRIPTION.md) per modello, licenze e misure
del solo Whisper. La RAM della GUI durante una call reale non è misurata.

## Verifiche

I test di layout e segmentazione sono distinti dalle prove su file audio e
dalla diagnostica del server AI. I dettagli della build corrente sono nel
[record dell'iterazione](review/iteration-11/VALIDATION.json). Le frasi italiane
di prova sono sintetiche: non certificano l'accuratezza della tua call Discord.
Il collaudo non richiede microfono o registrazione della tua chiamata.

Fonti: [process loopback Microsoft](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/),
[ducking](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-registerducknotification),
[identità della sessione](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiovolumeducknotification-onvolumeducknotification),
[whisper.cpp](https://github.com/ggml-org/whisper.cpp).
