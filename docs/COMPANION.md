# NotchLite Companion: Android e iPhone

Le app native in `companion/` sono client del protocollo locale descritto in
[PHONE-SHARE.md](PHONE-SHARE.md). Interfaccia scura Graphite; scanner locale e
campo per incollare il QR. Non usano account, cloud, telemetria o Bluetooth.
Questa anteprima conserva associazione e token soltanto in memoria.

## Come provarle

1. PC e telefono devono essere sulla stessa LAN, con un indirizzo IPv4 privato.
   Nella pagina **Telefono** di NotchLite avvia la sessione locale.
2. Apri il companion, scansiona il QR del PC oppure incollane il testo.
3. Premi **Associa al PC**. Confronta il codice mostrato dai due dispositivi e
   approva sul PC soltanto se coincide. L'approvazione scade dopo 120 secondi.
4. Per inviare usa **Invia un file al PC**, scegli un file e approva l'offerta
   sul PC. Per ricevere scegli **Ricevi e salva** e una destinazione locale.
   Un file può essere grande al massimo 256 MiB. È consentito un trasferimento
   per direzione. **Annulla trasferimento** interrompe l'operazione corrente.
5. Tieni il companion aperto durante le operazioni. Se passi a un'altra app,
   polling e operazioni di rete vengono interrotti; non c'è un servizio in
   background. Il ritorno in primo piano può interrogare la sessione ancora
   in memoria, senza eseguire una nuova associazione automatica.

La conferma al PC parte dopo la chiusura riuscita della destinazione Android
o dopo il completamento dell'esportazione iOS. Se il file è salvato ma la
conferma di rete fallisce, usa **Conferma file già salvato**; non occorre
scaricarlo una seconda volta. Una ricezione annullata non invia la conferma.
Android tenta di rimuovere il documento parziale appena creato, ma un provider
di documenti che rifiuta la cancellazione può richiedere la rimozione manuale.

## Android: APK installabile

Requisito minimo Android 8 (API 26). Nel workflow **Native companions**, il job
Android esegue i test del parser e produce l'artefatto
`NotchLite-Companion-Android-debug`, contenente l'APK di anteprima. Il debug APK
è firmato con la chiave debug del runner, non con una chiave di distribuzione:
build provenienti da runner diversi possono richiedere la disinstallazione
dell'anteprima precedente prima di installare la successiva.

Scarica l'APK da una build riuscita del repository, trasferiscilo al telefono
e aprilo; Android richiede di consentire l'installazione alla sorgente scelta.
La fotocamera serve soltanto allo scanner ed è facoltativa: l'inserimento
manuale funziona senza permesso fotocamera. Non occorrono permessi generici
di accesso ai file: selezione e salvataggio usano Storage Access Framework.

Per compilare da sorgente: JDK 17, Gradle 8.9 e Android SDK 35.
Il progetto fissa AGP 8.7.3 e Kotlin 2.0.21. Dalla cartella `companion/android`:

```sh
gradle :app:testDebugUnitTest :app:assembleDebug --no-daemon
```

Le dipendenze runtime sono AndroidX, Kotlin/coroutines, ZXing Android Embedded
e ZXing core; non è presente uno scanner cloud. Il pulsante **Licenze open
source** mostra `companion/NOTICE`, la licenza Apache completa e gli avvisi
presenti negli archivi JAR/AAR risolti dalla build. Il sorgente iOS non aggiunge
dipendenze esterne ai framework Apple.

## iPhone: sorgente Xcode, artefatto simulatore

Requisito minimo iOS 17. Apri
`companion/ios/NotchLiteCompanion.xcodeproj` su un Mac con Xcode, seleziona il
target `NotchLiteCompanion` e configura il tuo team di firma per eseguirlo
su un iPhone. L'installazione fisica dipende dalla firma e dal provisioning
Apple del tuo account. Questa anteprima non contiene un profilo di firma o
una distribuzione TestFlight/App Store.

Il job CI macOS compila senza firma per **iOS Simulator** e pubblica
`NotchLite-Companion-iOS-simulator-unsigned`, con bundle `.app` in un archivio
tar.gz che conserva permessi e collegamenti. Questo `.app` non è un IPA
installabile su iPhone. Una compilazione del simulatore non verifica la
fotocamera o il trasferimento sulla LAN di un dispositivo fisico.

```sh
swift test --package-path companion/ios
xcodebuild -project companion/ios/NotchLiteCompanion.xcodeproj \
  -scheme NotchLiteCompanion -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build
```

## Confini implementati e verifiche

Il parser accetta solo `notchlite://pair` versione 1 con parametri unici e
completi, IPv4 RFC1918 letterale, porta valida e pin/segreto esadecimali di
64 caratteri. Rifiuta DNS, indirizzi pubblici, loopback, multicast, URL con
userinfo/percorso/frammento, parametri duplicati e input oltre 2 KiB.
Le fixture dei test contengono soltanto valori sintetici ripetuti.

Le connessioni usano esclusivamente HTTPS sull'host/porta del QR. Il pin
SHA-256 del certificato DER leaf viene verificato nel callback TLS prima di
inviare il segreto. Sono mantenuti controllo di validità e identità IP del
certificato; il certificato del PC deve includere l'indirizzo LAN nei SAN.
Android usa un SSLContext per istanza senza modificare trust manager o
hostname verifier globali. iOS ancora la singola valutazione TLS al leaf
pinned, invoca la valutazione TLS della piattaforma con policy SSL per
quell'host e disabilita fetch di rete per la valutazione del certificato.
Le app rifiutano redirect HTTP. Pin errato, date e identità IP devono
essere verificati anche contro i runtime reali, oltre alla revisione sorgente.

Le risposte JSON sono limitate a 64 KiB. Upload e download sono stream su
disco, con dimensione nota e limite 256 MiB; un download con Content-Length
diverso dall'offerta fallisce. iOS prepara una copia temporanea a blocchi del
file selezionato per offrire a URLSession una sorgente immutabile con lunghezza
esatta; viene eliminata dopo l'operazione. Nessun file completo viene caricato
in un buffer applicativo. Il token non viene salvato in preferenze, Keychain
o backup. Non viene pubblicato un deep link di associazione automatico: la
connessione richiede un'azione esplicita nell'app.

I test puri Android e Swift verificano il parser, compresi endpoint non
ammessi, duplicati e credenziali malformate. Il workflow compila entrambi i
client. I risultati effettivi dei run CI sono l'evidenza di compilazione:
questo documento non dichiara superati job ancora in esecuzione.
Fotocamera reale, permessi LAN iOS, TLS e trasferimenti tra dispositivi,
provider file, interruzioni e consumi richiedono prove successive sui telefoni.
Non sono state eseguite prove GUI o installazioni su dispositivi in questa
iterazione, come richiesto dall'utente.

## Fonti tecniche

- [Compatibilità AGP / Gradle](https://developer.android.com/build/releases/about-agp).
- [ZXing Android Embedded 4.3.0](https://github.com/journeyapps/zxing-android-embedded/releases/tag/v4.3.0).
- [ZXing core e notice](https://github.com/zxing/zxing/tree/zxing-3.5.3).
- [Gradle GitHub Actions](https://docs.gradle.org/current/userguide/github-actions.html).
- [Apple: certificati di ancoraggio TLS](https://developer.apple.com/documentation/security/sectrustsetanchorcertificates(_:_:)).
- [Apple: sicurezza connessioni](https://developer.apple.com/documentation/security/preventing-insecure-network-connections).
