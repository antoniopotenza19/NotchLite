# Server Codex locale condiviso

Verifica: 26 novembre 2026, Windows x64, CLI ufficiale `0.171.0-alpha.13.2`.

La soluzione scelta è il **daemon ufficiale Codex**, avviato esplicitamente. NotchLite e il CLI possono usare lo stesso server locale; chiudere il pannello non deve terminare le sue chat. Questo non migra le chat già attive nel server incorporato di Codex Desktop. La presenza delle stesse conversazioni salvate non dimostra che gli eventi della GUI siano condivisi.

## Pacchetto verificato

Il `codex.exe` incluso nel pacchetto Desktop riconosce i comandi ma, in questa installazione, `daemon start` ha rifiutato l'avvio perché mancava un pacchetto locale completo riconosciuto. Non è stato aggirato il controllo costruendo manifest o copiando helper a mano.

È stato scaricato ed estratto soltanto nella cartella di lavoro il [pacchetto completo della stessa versione](https://github.com/openai/codex/releases/tag/rust-v0.171.0-alpha.13.2):

- Asset: `codex-package-x86_64-pc-windows-msvc.tar.gz`, 215.183.225 byte.
- [Download ufficiale](https://github.com/openai/codex/releases/download/rust-v0.171.0-alpha.13.2/codex-package-x86_64-pc-windows-msvc.tar.gz).
- SHA-256 verificato prima dell'estrazione: `6d3be3ea2e3f759f77f23b543ee882aae6433dc6a6cd260788c2e8793d616eac`.
- CLI estratto: `codex-cli 0.171.0-alpha.13.2`; SHA-256 dell'eseguibile: `3b0b6d91b32a43b6e708a200445a7dff190b0116009ca29c3bca3623a1bbb69e`.
- Sorgente del tag: commit `ad9d659a8491fc5169a00dcf7b7e6ca8a87adf69`.

Il layout include `codex-package.json`, `bin/codex.exe`, `bin/codex-code-mode-host.exe`, `codex-path/rg.exe` e gli helper Windows sotto `codex-resources/`. È il layout richiesto dal [validatore upstream](https://github.com/openai/codex/blob/ad9d659a8491fc5169a00dcf7b7e6ca8a87adf69/codex-rs/app-server-daemon/src/prepare_install.rs), non il solo binario CLI.

Lo script locale riproducibile è `work/iteration-8/setup-cli-package.ps1`, relativo alla radice della chat di sviluppo. Verifica digest, nomi/tipi delle voci dell'archivio, manifest, helper e `--version`; non avvia server, login o installer. I metadati sono in `work/iteration-8/setup-cli-metadata.json`. Non è stato modificato il PATH globale.

## Avvio esplicito

Dalla radice del repository, l'helper pubblico scarica il pacchetto fissato, controlla SHA-256, archivio, manifest, helper e versione. L'avvio richiede l'opzione esplicita:

```powershell
.\tools\Setup-CodexSharedServer.ps1 -Start
```

Senza `-Start` prepara e verifica soltanto il pacchetto, stampando il percorso del CLI. La destinazione predefinita è `%LOCALAPPDATA%\NotchLite\codex-setup`; `-DestinationRoot` può scegliere un'altra cartella locale assoluta. Questo helper viene lanciato manualmente: l'avvio normale di NotchLite non scarica dipendenze.

Dalla cartella portatile che contiene `notchlite.exe`, i controlli headless sono:

```powershell
.\notchlite.exe --codex-server-start | Out-Host
.\notchlite.exe --codex-connect-check | Out-Host
```

Il primo avvia il daemon ufficiale o rileva quello già in esecuzione. Il secondo collega davvero il Bridge e verifica account ChatGPT e disponibilità dei limiti, senza avviare un turno o inviare messaggi. Entrambi terminano prima di inizializzare la GUI, il profilo o COM. Un risultato di sola presenza del socket (`--codex-check`) rimane distinto da questa connessione reale.

La risoluzione del CLI preferisce il pacchetto gestito sotto `CODEX_HOME/packages/app-server-daemon/current/bin/codex.exe`, poi l'eventuale standalone, prima dell'alias Desktop. Per una diagnosi con un eseguibile scelto esplicitamente è disponibile `--codex-cli <percorso-assoluto>`.

Il controllo del metodo di autenticazione deve indicare ChatGPT per rispettare la scelta dell'abbonamento. Non servono lettura/copia di token o chiavi API. L'autenticazione del CLI è gestita dal client ufficiale; non viene cambiato `CODEX_HOME` per creare una seconda identità. Non vengono eseguiti login, logout o migrazioni automatiche. Le [modalità ufficiali di autenticazione](https://learn.chatgpt.com/docs/auth#openai-authentication) distinguono l'accesso tramite abbonamento dall'uso a consumo con API key.

Il primo avvio riuscito può copiare il pacchetto nella directory utente `CODEX_HOME/packages/app-server-daemon/` e creare lo stato del daemon. È una modifica locale esplicita del gestore ufficiale, distinta dalla semplice estrazione sotto `work/`. Non è un servizio Windows o un avvio automatico al login configurato da NotchLite.

Nella prova reale del coordinatore, `start` ha restituito stato `started`; versioni CLI, pacchetto gestito e server coincidevano con `0.171.0-alpha.13.2`. Il gestore ha selezionato `packages/app-server-daemon/current/bin/codex.exe` e il socket canonico `app-server-control/app-server-control.sock` nella normale home Codex. Nessun PID, dato di chat o credenziale viene pubblicato qui.

Per verificare manualmente il metodo di autenticazione con quel CLI, senza aprire file di credenziali:

```powershell
$codexStateRoot = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $env:USERPROFILE '.codex' }
$codexCli = Join-Path $codexStateRoot 'packages/app-server-daemon/current/bin/codex.exe'
& $codexCli login status
```

## Collegamento dei client

Il [trasporto Unix ufficiale](https://learn.chatgpt.com/docs/app-server#protocol) usa WebSocket su socket locale. Il Bridge di NotchLite impiega il proxy ufficiale:

```text
codex.exe app-server proxy --sock <socket-canonico>
```

Il CLI può essere indirizzato esplicitamente allo stesso server:

```powershell
$projectDir = (Get-Location).Path
& $codexCli --remote unix:// -C $projectDir
# In alternativa, il centro sessioni terminale:
& $codexCli agents --remote unix://
```

Questi ultimi comandi aprono client terminali: non sono stati eseguiti durante la preparazione del pacchetto. Gli [argomenti ufficiali del CLI](https://learn.chatgpt.com/docs/developer-commands) distinguono il collegamento `--remote` dal server incorporato. Le chat devono risiedere sul server condiviso per riceverne eventi e domande; nessun riavvio di Desktop o accesso ai suoi database viene usato per simularlo.

Su Windows il percorso canonico del socket deve stare nel limite AF_UNIX di 107 byte utili. Non usare una junction corta per mascherare un percorso risolto troppo lungo. In questa prova il percorso canonico normale era utilizzabile, quindi non è stato introdotto un socket alternativo o un listener TCP/LAN.

## Proprietà e arresto

Il daemon gestisce il proprio stato e la propria identità di processo. NotchLite possiede soltanto il suo client/proxy: alla chiusura del pannello deve rilasciare quelli, lasciando il server e gli altri client attivi. `start` è idempotente; un server trovato già avviato non diventa un processo da terminare perché si chiude la dashboard.

Per fermare intenzionalmente il server condiviso:

```powershell
& $codexCli app-server daemon stop
```

L'arresto riguarda tutti i client e può interrompere lavoro attivo: non è parte della normale chiusura del pannello. Non sono previsti `restart`, `update`, `bootstrap` o abilitazione del controllo remoto automatici. La versione alpha non è un canale stable/latest secondo la logica upstream; ciò non è una promessa di assenza di rete o aggiornamenti per ogni futura configurazione. Vedere il [lifecycle ufficiale del daemon](https://github.com/openai/codex/blob/ad9d659a8491fc5169a00dcf7b7e6ca8a87adf69/codex-rs/app-server-daemon/README.md).

## Prove reali eseguite

Il coordinatore ha verificato, senza GUI o audio:

- Avvio ufficiale `started`, poi rilevamento idempotente `alreadyRunning`.
- `notchlite.exe --codex-server-start`: server condiviso disponibile.
- `notchlite.exe --codex-connect-check`: connessione reale, account ChatGPT e limiti disponibili.
- Il test end-to-end finale è riuscito con una chat di prova e due prompt: un primo turno prepara la chat, quindi un secondo turno verifica la condivisione con entrambi i client già sottoscritti. Entrambi hanno osservato la stessa chat, risposta, completamento e metadati di utilizzo token; è riuscito anche il recupero dalla cronologia del completamento del primo turno. La verifica non ha aperto una GUI.

Non vengono pubblicati ID, contenuto del prompt, risposta, valori dei token o credenziali. La prova riguarda una chat del nuovo server condiviso; le chat già attive nella GUI Desktop non sono state collegate o migrate.

La verifica end-to-end è ripetibile solo mediante opzione esplicita e **crea una chat/invia due prompt usando i limiti dell'abbonamento**. Da una cartella portatile:

```powershell
$testWorkspace = Join-Path $env:TEMP ('NotchLite-codex-check-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testWorkspace | Out-Null
.\notchlite.exe --codex-self-test --workspace $testWorkspace | Out-Host
```

La directory esplicita è dedicata alla prova. Il normale avvio e il controllo di connessione non creano chat o messaggi. Questo esito non verifica resa dei pannelli, input nativi, comportamento su due monitor o acquisizione audio; tali prove restano separate.
