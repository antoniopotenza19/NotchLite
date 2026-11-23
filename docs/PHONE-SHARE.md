# Collegamento al telefono — Android e iPhone

**Stato: progetto del flusso, non pairing implementato nella 0.1.5.** Nessun listener LAN, servizio, dispositivo associato o trasferimento reale viene aggiunto da questa versione.

Il riferimento pubblico è [Noci Share di faisyalrizki](https://www.instagram.com/p/DgEXHeggzMe/): il video mostra QR nella notch, apertura dal telefono, scelta di foto, accettazione sul PC e invio inverso di un file. La schermata finale dichiara stessa Wi-Fi e nessuna app da installare. È una dimostrazione video: protocollo, cifratura, codice originale e compatibilità effettiva non sono stati verificati. Non è stato identificato un repository originale riutilizzabile.

La richiesta comprende **Android e iPhone**. Il primo obiettivo è lo scambio file tramite il browser del telefono, distinto da Bluetooth, telecomando completo, notifiche/SMS o controllo del PC in background. Non serve un APK per la proposta web.

## Flusso da realizzare

1. L'utente apre la sezione Telefono e avvia un collegamento temporaneo; il modulo è inizialmente spento.
2. Il PC mostra il QR della sessione, una scadenza breve e Annulla. Rigenerare il QR invalida il precedente.
3. Il browser del telefono apre il client. Prima di autorizzare operazioni, il PC mostra una richiesta di consenso legata a quella sessione.
4. Solo dopo la conferma si abilita lo scambio. Il telefono sceglie i file; il PC vede nome e dimensione e può accettare o rifiutare.
5. Il PC può selezionare o trascinare un file per il telefono. Entrambi vedono progresso, annullamento e completamento reale.
6. Disconnessione o revoca interrompono la sessione e le operazioni pendenti. Un risultato tardivo non può riattivare una sessione chiusa.

Gli stati devono distinguere richiesta di associazione, autorizzazione, collegamento attivo e trasferimento: un QR mostrato non significa telefono connesso. Nessun pulsante o indicatore simulerà una riuscita.

## Punto da risolvere prima del backend

**Il browser deve ricevere il client e i file da un'origine autenticata.** Un QR con un URL HTTP nella LAN non protegge il contenuto o il codice della pagina da una rete ostile. Un server HTTPS con certificato autofirmato non diventa automaticamente fidato su Safari o Chrome. Il pinning che un'app nativa può implementare non è una funzione ottenuta dal solo QR nel browser.

Non verranno usati bypass degli avvisi TLS o installazioni di certificati come normale onboarding. Prima di scegliere il listener occorre una prova interoperabile di origine HTTPS e accesso alla LAN, oppure un canale WebRTC con distribuzione del client e segnalazione autenticate. Quest'ultima strada introduce componenti e costi di esercizio da valutare; non è dichiarata pronta. Si devono verificare anche le restrizioni di accesso alla rete locale dei browser.

Il modello [PC Deck di NotchPixel](https://notchpixel.com/deck) usa app Android/iOS, QR temporaneo, conferma sul PC e identità del PC verificata: è un riferimento distinto, non prova che lo stesso schema funzioni senza companion nativo. [KDE Connect](https://kdeconnect.kde.org/) è un'alternativa con applicazioni separate e limiti iOS, non un'implementazione del flusso web Noci; non viene installato o incorporato.

## Confini e costo previsti

- Modulo esplicitamente opt-in, anche per i profili esistenti. Nessuna ricerca implicita nel costruttore della pagina e nessun I/O nel disegno.
- Sessione con segreto casuale forte, scadenza monotona, uso singolo e conferma PC. Il solo indirizzo IP o la presenza sulla stessa Wi-Fi non autorizzano il telefono.
- Trasferimenti in streaming con buffer limitati, limiti a dimensioni/concorrenza/rate e annullamento. Non caricare un intero file nella RAM del PC.
- Accettazione di ogni trasferimento, nomi file trattati come dati, cartella destinazione controllata e nessuna sovrascrittura o apertura automatica.
- Nessun comando shell, accesso remoto arbitrario ai file, sincronizzazione continua degli appunti, SMS, contatti o input remoto nel primo incremento.
- Spegnere il modulo chiude listener/worker, cancella le richieste e rilascia le risorse. I costi del provider e del browser del telefono vanno misurati separatamente e insieme.

## Verifiche necessarie

Prima del rilascio del backend: scadenza/riuso QR, conferma per la sessione esatta, rifiuto delle azioni prima del consenso e dopo revoca, cancellazione a metà file, limiti dei buffer, nomi ostili, errori disco, sessioni concorrenti e conservazione delle impostazioni. Poi prove reali con Safari su iPhone e Chrome su Android, rete privata/isolata, sospensione della pagina e cambi Wi-Fi.

Questa iterazione prepara la UI nativa e la specifica. Non usa il desktop, non avvia NotchLite e non apre porte di rete sul PC.
