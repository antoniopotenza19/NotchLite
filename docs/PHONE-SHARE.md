# Telefono: QR e file sulla rete locale

La 0.1.6 introduce una preview con companion native Android e iPhone. La funzione è spenta inizialmente anche nei profili esistenti. L'interruttore rende disponibile la pagina; il server parte solo con **Start QR pairing**. Compilazione e test di protocollo sono distinti dalle prove con telefoni reali, ancora necessarie.

Il riferimento visivo è [Noci Share](https://www.instagram.com/p/DgEXHeggzMe/). Protocollo e app sono implementazioni indipendenti. L'utente ha scelto app native e sola LAN: niente tunnel, relay o cloud. Il QR si scansiona nella companion, non in Safari/Chrome.

## Uso

1. In Settings abilita **Phone link (LAN)**; scegli **Graphite** in Appearance.
2. Apri Telefono. **Choose network** elenca gli IPv4 privati del PC: scegli quello della stessa rete del telefono se ne compare più di uno. Nessuna ricerca della rete parte nel costruttore della pagina.
3. **Start QR pairing** apre la finestra QR. Scansionalo nella companion. Segreto monouso; approvazione entro due minuti dall'avvio.
4. Confronta il codice su telefono e PC, poi premi **Codes match: allow** sul PC. Il nome del telefono da solo non prova l'identità.
5. Dal telefono scegli un file: il PC mostra nome e dimensione e attende **Accept file**. Dal PC seleziona Telefono e trascina un file; la companion permette di scaricarlo e salvarlo.
6. **Stop/Disconnect**, disabilitare la funzione o uscire revoca la sessione. Chiudere la finestra QR annulla la sessione in attesa al successivo tick della UI.

Ricevuti su PC: `%LOCALAPPDATA%\NotchLite\Received`, in sottocartelle per trasferimento, senza apertura automatica o sovrascritture. Un file in entrata e uno in uscita, massimo 256 MiB ciascuno. Cartelle e selezioni multiple non supportate.

Servono dispositivi sulla stessa rete raggiungibile. Reti ospiti isolate, VPN o firewall possono bloccare il collegamento. Non vengono aggiunte automaticamente eccezioni al firewall di Windows. App Windows e telefoni non vengono aperti durante questa preparazione.

## Sessione e risorse

Il PC genera certificato TLS temporaneo e segreto casuale di 256 bit. Il QR contiene IPv4 privato, porta, impronta SHA-256 DER e segreto. La companion verifica il pin prima di inviare credenziali e rifiuta redirect, DNS e IP pubblici nel QR. Il primo invio consuma il segreto; crea ID, token di attesa distinto e codice di confronto. Solo il consenso all'esatto ID sul PC rilascia il token di sessione.

È una sessione temporanea con pin e bearer token, non mTLS o associazione permanente. I segreti restano in memoria. Dopo riavvio/Stop serve un nuovo QR. I due minuti riguardano l'associazione; la sessione approvata dura fino a revoca/uscita. Ogni richiesta chiude la propria connessione HTTP: ciò non revoca il token. Perdita del Wi-Fi o chiusura improvvisa della companion non equivalgono a Stop sul PC.

File in streaming, buffer limitati, consenso prima dell'upload, nomi validati come singoli nomi Windows e creazione senza sovrascritture. Nessun file intero caricato nella RAM del PC. Stop revoca la sessione e chiude listener/socket; l'attesa sulla UI è limitata. Un I/O disco bloccato dal sistema può completare più tardi. Una rinomina già consegnata al sistema può concludersi anche dopo Stop; le operazioni non ancora avviate vengono annullate. Non promettiamo tempi fissi di rilascio di ogni risorsa kernel.

Non aggiunge shell remota, esplorazione arbitraria dei file, appunti, SMS, contatti o controllo del PC. Il QR non installa l'app iPhone: su dispositivo serve una build firmata. Una build per simulatore non è installabile su iPhone. Vedi [companion e build](COMPANION.md).

## Bluetooth

Non implementato in questa preview. Possibile incremento successivo per individuazione/abbinamento e piccoli messaggi via BLE. Lo scambio qui descritto richiede la LAN. Un canale Bluetooth senza Wi-Fi richiede prove su driver/adattatore Windows e dispositivi Android/iPhone, con velocità e consumi misurati.

Fonti: [Android BLE](https://developer.android.com/develop/connectivity/bluetooth/ble/ble-overview), [Apple Core Bluetooth](https://developer.apple.com/documentation/corebluetooth), [Windows GATT server](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/gatt-server).

## Contratto v1

QR: `notchlite://pair?v=1&host=<IPv4 privato>&port=<porta>&pin=<SHA256 DER hex>&secret=<256 bit hex>`.

HTTPS, Content-Length, nessun redirect/Transfer-Encoding, una connessione per richiesta. Header massimo 16 KiB, JSON in ingresso al PC massimo 8 KiB.

| Endpoint | Autorizzazione / risultato |
|---|---|
| POST /v1/pair | JSON secret,name; restituisce id,pollToken,code |
| GET /v1/pair/{id} | Bearer pollToken; pending/approved/denied/expired, token sessione solo dopo approvazione |
| GET /v1/state | Bearer sessione; stato e offerte correnti |
| POST /v1/offers | Bearer sessione, JSON name,size; ID offerta in entrata |
| PUT /v1/incoming/{id} | Bearer sessione, file raw della dimensione esatta; solo dopo consenso PC |
| GET /v1/outgoing/{id} | Bearer sessione; file offerto esplicitamente dal PC |
| POST /v1/outgoing/{id}/done | Bearer sessione; conferma download e salvataggio |
| DELETE /v1/transfers/{id} | Bearer sessione; annulla l'offerta esatta |

Restano necessarie prove reali di scanner, consenso rete locale iOS, firewall, scambio bidirezionale, sospensione/cambio rete, errori disco, resa/DPI e RAM/CPU. I test sintetici non le sostituiscono.
