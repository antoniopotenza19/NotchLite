# Revisione dei flussi call

Revisione statica del 26 novembre 2026, con il metodo Ponytail del progetto.
Carico considerato: una sola sessione di ascolto confermata, un'applicazione
selezionata e una trascrizione CPU progressiva. Non sono stati avviati
microfono, acquisizione audio, applicazioni grafiche o chiamate dal revisore.

## Revisione indipendente

L'autore del ponte opzionale `call_coach` ha letto i moduli di altri autori:
`call_audio.rs`, `call_audio/windows.rs`, `call_audio/capture.rs`,
`call_stt.rs` e il coordinatore `call_session.rs`, inclusi i chiamanti e i test.

Sono stati seguiti conferma, stato Starting, identità PID/tempo di creazione/
sessione, callback di attivazione, lettura PCM, generazioni, arresto, coda STT,
file temporanei e gestione dei risultati tardivi. La cattura usa il processo
selezionato e i suoi figli; non introduce ripiego sul microfono o sull'audio
globale. I callback mantengono vivi i parametri immutabili; l'accesso alle
interfacce di acquisizione resta sul worker MTA. I pacchetti nativi e i
processi STT hanno un proprietario e limiti espliciti.

Due problemi concreti sono stati segnalati e la correzione è stata riletta:

- Una risposta parlata breve dopo una pausa poteva essere scartata dal minimo
  di un secondo imposto al segmento. Il segmento con voce sufficiente viene
  ora mantenuto e completato con silenzio fino alla lunghezza minima STT.
- La configurazione STT veniva aperta prima del controllo del percorso
  locale. `runtime.json` viene ora controllato come file regolare, con
  verifica delle directory dalla radice verso il file, prima della lettura.
  Questa sequenza evita di attraversare una junction prima di rifiutarla.

Non sono rimasti rilievi concreti aperti nel perimetro letto. Questo esito
non dimostra la compatibilità WASAPI sul dispositivo dell'utente né la
precisione della trascrizione italiana su chiamate reali.

## Ponte opzionale: attribuzione della verifica

Il codice `call_coach.rs` è dell'autore di questo rapporto: le sue verifiche
dirette sono quindi un autocontrollo. Un secondo autore ha segnalato in modo
indipendente il rischio di blocco su un pacchetto locale incompleto. Il
timeout per inattività non limitava la durata totale di `read_exact`.

La correzione usa una scadenza assoluta per l'intero frame, controlla la
cancellazione tra le operazioni e limita le singole attese I/O. Un test
sintetico apre una richiesta incompleta e controlla che Stop non aspetti il
resto del payload. Sono controllati anche autenticazione locale, cursori
emessi, rifiuto dei replay, sessioni/generazioni, dimensioni JSON, percorsi
locali e rimozione del file di individuazione posseduto.

Il ponte non chiama Codex né altre API e non interpreta comandi. Richiede
la partecipazione attiva dell'assistente in una chat; una chat inattiva non
produce automaticamente suggerimenti. È separato dalla trascrizione locale
e resta opzionale.

## Evidenza e limiti

Il riepilogo locale del coordinatore consultato registra 128 test passati e
zero fallimenti. Il revisore non ha eseguito quei test o la build completa;
il record di validazione del coordinatore resta l'autorità per l'ultima
compilazione e le prove di integrazione.

Restano da verificare ascolto reale confermato, resa e scorrimento della UI,
focus/DPI/hotplug, accuratezza su interlocutori e ambienti diversi e consumo
complessivo della GUI durante la chiamata. I limiti delle chiamate di
sistema non sono una garanzia assoluta sui tempi del kernel.
