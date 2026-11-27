# Trascrizione locale delle call

Il modulo `call_stt` usa **whisper.cpp locale, CPU o Vulkan esplicito** per convertire brevi segmenti
audio in testo sul PC. Non apre microfoni, sessioni WASAPI, browser o client
di rete. L'acquisizione dell'app selezionata e l'invio del testo a Codex sono
responsabilità distinte del controller della call.

## Preparazione

Da PowerShell, nella cartella del progetto o del pacchetto portatile:

```powershell
.\tools\Setup-CallTranscription.ps1
# Profilo più piccolo, con lo stesso rilevatore vocale:
.\tools\Setup-CallTranscription.ps1 -Model tiny
# Scelte esplicite: modello, backend e thread del processo ASR.
.\tools\Setup-CallTranscription.ps1 -Model small -Backend cpu -Threads 4
# Candidato GPU da validare sul proprio hardware, senza attivare l'ascolto:
.\tools\Setup-CallTranscription.ps1 -Model turbo -Backend vulkan -Device 0 -Threads 4
```

Il setup propone **Small Q5_1 + Silero VAD su CPU, quattro thread**, con italiano esplicito.
Tiny, Base e Large v3 Turbo restano selezionabili. Small e Turbo scaricano
rispettivamente circa 190 MB e 574 MB di pesi: la dimensione su disco non è
una misura della RAM durante l'inferenza. `-Threads` accetta da uno a otto;
`-Device` sceglie un indice GPU e viene usato soltanto con Vulkan. Non c'è
selezione automatica basata sul nome della scheda o sui valori WMI.
`-Language en` o `-Language auto` sono disponibili se la call usa un'altra
lingua. Il setup aggiorna `runtime.json` nella sola cartella NotchLite.
Una sessione già confermata conserva la configurazione iniziale: per usare
il profilo aggiornato occorrono Stop e una nuova conferma. Non riparte da sola.

Lo script scarica dipendenze fissate e verifica SHA-256 **prima** di estrarre
o usare i file. Prepara `%LOCALAPPDATA%\NotchLite\whisper\runtime.json`;
non modifica PATH globale, avvio con Windows o impostazioni audio. Il setup
non avvia l'ascolto o un'inferenza. Richiede Windows x64 e una cartella locale
senza junction o altri reparse point nei componenti del percorso.

Con `-Backend cpu` il runtime è quello ufficiale [whisper.cpp v1.9.2](https://github.com/ggml-org/whisper.cpp/releases/tag/v1.9.2),
asset `whisper-bin-x64.zip`, circa 8,2 MB compressi. Con `-Backend vulkan`
viene preparato il pacchetto [Subtitle Edit whispercpp-195](https://github.com/SubtitleEdit/support-files/releases/tag/whispercpp-195),
asset `whisper-vulkan-x64.zip`, circa 13 MB compressi: è una build di terzi
del progetto whisper.cpp, il cui CLI riporta **1.9.5-dev**. Non è un binario
pubblicato da ggml-org. Il setup non esegue nessuno dei due runtime.

Il CLI e le DLL di ogni backend vengono estratti con whitelist distinta in
una nuova cartella. Non vengono mescolate librerie CPU/Vulkan o librerie
di installazioni precedenti. I modelli multilingue provengono dal
[repository originale dei modelli convertiti](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1).
Small Q5_1 occupa 190.085.487 byte; Turbo Q5_0 574.041.195 byte;
Base Q5_1 59.707.625 byte e Tiny Q5_1 32.152.673 byte.
Si aggiunge il rilevatore vocale `ggml-silero-v6.2.0.bin`, 885.098 byte,
dal [repository VAD del progetto](https://huggingface.co/ggml-org/whisper-vad/tree/9ffd54a1e1ee413ddf265af9913beaf518d1639b).
I testi MIT separati di whisper.cpp, del modello Whisper e di Silero sono conservati accanto al
runtime. Questi componenti opzionali restano processi/file separati, senza
collegamento della libreria Whisper nel processo della notch.

| File | SHA-256 |
| --- | --- |
| `whisper-bin-x64.zip` | `49dcc16de826f20bd53d44f947a1ae49dfa81f86cad67a64d80820cb192d674a` |
| `whisper-vulkan-x64.zip` (`whispercpp-195`) | `89f2c36455939ad42e08b2107bd6b47105b6d9c1a2b0916cbb500ec27deb520a` |
| `ggml-base-q5_1.bin` | `422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898` |
| `ggml-tiny-q5_1.bin` | `818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7` |
| `ggml-small-q5_1.bin` | `ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb` |
| `ggml-large-v3-turbo-q5_0.bin` | `394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2` |
| `ggml-silero-v6.2.0.bin` | `2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987` |
| `whisper-cli.exe` CPU 1.9.2 | `95e3c0b0e778ad9499eb0125f97c1dcf437dd9eb4ea77050b043574f93c2631d` |
| `whisper-cli.exe` Vulkan `whispercpp-195` | `1ef8f16edf2f8df429e7e1d18c9af073f66d594813e9aae739e5bdf015a61985` |

Tiny/Base usano il pin `c521a4b02f422512d734391fdf08bb08c0862f68`;
Small/Turbo usano `5359861c739e955e79d9a303bcbc70fb988958b1`.
Una nuova preparazione crea una directory runtime distinta; i vecchi file
non vengono cancellati implicitamente.

Il manifest conserva `backend` (`cpu` o `vulkan`), `device` e `threads`,
oltre ai campi precedenti. I manifest precedenti restano compatibili:
l'assenza di questi campi mantiene CPU e due thread. Per usare il nuovo
profilo servono Stop e una nuova conferma; il setup non cambia una sessione
già in corso. I runtime e i modelli non sono inclusi nell'eseguibile o nel
pacchetto portatile.

### Provenienza e limiti Vulkan

La build Subtitle Edit è tracciata al
[workflow pubblico fissato](https://github.com/SubtitleEdit/support-files/blob/358047ebca4410be9636974795e68e01427b6f53/.github/workflows/build-whispercpp-release.yml)
e alla relativa [esecuzione riuscita](https://github.com/SubtitleEdit/support-files/actions/runs/38219803908).
Il workflow compila da un ref upstream fornito all'avvio con Vulkan attivo,
SDL2 disabilitato e librerie condivise. Il tag upstream v1.9.5 risolve al
commit `d1be6fde11ac6e0407606b4e42fe72d34add8037`. Questo legame e il pin
del pacchetto non costituiscono una riproduzione indipendente della build
o un'attestazione crittografica sorgente-binario.

Il profilo Vulkan richiede `--no-flash-attn` esplicito. L'upstream documenta
[crash con Flash Attention su RX 9070 XT](https://github.com/ggml-org/whisper.cpp/issues/3806);
la versione 1.9.5 include anche correzioni per
[scritture oltre il limite della memoria condivisa](https://github.com/ggml-org/whisper.cpp/commit/cdc873029c1f4e58356914b7ab791d295897f5b5)
e [riuso di buffer temporanei](https://github.com/ggml-org/whisper.cpp/commit/82fa9df72991472c5a775e21944887122f2c39d1).
Queste fonti motivano la scelta prudente; non dimostrano la causa del crash
intermittente osservato nel candidato 1.9.2 né garantiscono stabilità sul
PC dell'utente. La validazione del candidato 1.9.5 resta distinta dalla
preparazione dei file.

Una scheda con driver Vulkan compatibile può eseguire questo runtime senza
installare un SDK. Il setup non installa SDK o driver. “Vulkan richiesto”
indica una configurazione: occorre verificare che l'inferenza usi davvero
la GPU scelta. L'indice può cambiare con hardware, driver o override di
ambiente; non assumere che GPU 0 identifichi sempre la scheda dedicata.
La GPU può cambiare la latenza e il consumo; l'accuratezza dipende anche
dal modello e dall'audio e deve essere misurata sugli stessi campioni.

Nella 0.1.12 il worker rimuove dal solo ambiente del processo figlio gli
override `GGML_VK_*`, `GGML_VULKAN_*` e `WHISPER_ARG_*`, poi imposta device e
no-flash esplicitamente. Legge soltanto i metadati iniziali limitati del
runtime: allocazione modello sul device, backend selezionato e compute
buffer. Non salva stderr o testo riconosciuto nei log. Il VAD CPU successivo
non invalida questa conferma. Se manca la conferma e c'è testo, l'inferenza
fallisce: nessun risultato CPU è presentato come GPU. Silenzio e assenza di
voce possono legittimamente restituire testo vuoto senza conferma GPU.
La diagnostica `--call-transcription-check` espone `profile` e `gpuVerified`.

Il [confronto italiano della 0.1.12](review/iteration-13/TRANSCRIPTION.md)
ha verificato questo profilo sulla RX 9070 XT; non certifica altre GPU,
driver o qualità della call reale. Nel pannello il profilo proviene dalla
configurazione congelata della sessione, senza rileggere file durante il disegno.

## Funzionamento e limiti

`SttWorker::new` verifica la configurazione e resta inerte: niente thread o
modello caricato. Il primo segmento avvia un worker; ogni segmento viene
elaborato da un solo processo CLI nascosto, che termina prima del successivo.
Il modello viene quindi rilasciato tra le inferenze. La configurazione iniziale
usa italiano, quattro thread nel nuovo setup CPU, beam search con cinque
candidati e contesto audio completo (`audio-ctx 0`), con segmenti PCM16 mono
a 16 kHz lunghi al massimo otto secondi. È stato rimosso il profilo greedy
forzato con un solo candidato e senza fallback. Il rilevatore Silero ammette
voce da 100 ms e aggiunge margine audio da 160 ms, per mantenere anche risposte
brevi. Il runtime precedente senza `vadModel` rimane utilizzabile, con un
messaggio che invita ad aggiornare il setup.

Il silenzio digitale non avvia il modello. Dopo un taglio a sei secondi,
l'overlap trattenuto non conta come nuova voce: una pausa non viene più
inviata come un secondo segmento contenente soltanto overlap o zeri. Non
vengono applicate blacklist di parole o frasi riconosciute.

C'è spazio per un solo segmento in attesa oltre a quello in lavorazione.
Un nuovo segmento può sostituire quello in coda; `ReplacedQueued` permette
al controller di mostrare esplicitamente una lacuna. Anche il risultato ha
una sola posizione: `dropped_results` segnala risultati non letti sostituiti.
Il testo di ciascun risultato è limitato a 8 KiB. Un'inferenza oltre 30 secondi
termina il processo posseduto e restituisce un errore.

La generazione della cattura e l'identità del segmento vengono passate al
risultato senza modificarle. Il controller usa l'identità per distinguere
la sovrapposizione audio dalle parole ripetute realmente dall'interlocutore;
un epoch separato gestisce cancellazione e risultati tardivi. `stop` è
terminale: revoca l'epoch, svuota le code, interrompe il solo processo Whisper
posseduto e risveglia il worker. La UI attende al massimo circa 500 ms; una
chiamata al sistema può terminare più tardi mantenendo i propri riferimenti.

Per interfacciarsi al CLI, il segmento viene scritto in una directory TEMP
unica con ACL riservata a proprietario e SYSTEM. WAV e testo temporanei
vengono eliminati dal proprietario RAII al termine, anche su errore o Stop.
Un arresto improvviso del processo o del sistema può lasciare residui: non
viene promessa cancellazione fisica dei blocchi sul disco. Il worker non
scrive la trascrizione nei log e non mantiene una registrazione della call.

## Verifica ripetibile

Il modulo espone `check_file(path, language)` per una prova headless esplicita
su un WAV PCM16 mono 16 kHz di massimo otto secondi, con percorso locale
assoluto. Runtime, modello, fixture e directory TEMP rifiutano unità di rete
e reparse point. Il parser controlla
header, formato, lunghezze e dimensioni prima dell'inferenza. Questa funzione
non attiva alcun dispositivo audio o rete; l'integrazione CLI dell'app è
registrata separatamente dal coordinatore.

Il 26 novembre 2026 è stato eseguito il CLI reale sulla prima porzione di sei
secondi del [WAV pubblico fornito da whisper.cpp](https://github.com/ggml-org/whisper.cpp/blob/v1.9.2/samples/jfk.wav),
senza playback, microfono, GUI o cattura di call. La frase iniziale inglese è
stata riconosciuta. È una verifica funzionale limitata, non un test WER o una
prova di accuratezza su italiano, accenti, rumore o più interlocutori. La
tabella seguente descrive quel **profilo storico Tiny/contesto 512**, non il
nuovo profilo Base/VAD.

| Misura del solo processo Whisper | Risultato di una singola inferenza |
| --- | --- |
| Windows / thread CPU | build 26300 / 2 |
| Modello / contesto audio | tiny Q5_1 / 512 |
| Durata audio / tempo trascorso | 6 s / 0,309 s |
| Picco working set campionato | 96,12 MiB |
| Picco memoria privata impegnata campionato | 605,76 MiB |
| Tempo CPU campionato | 0,391 s |

Campionamento ogni 20 ms, nove campioni. Working set e memoria privata
impegnata sono misure differenti: il secondo valore non è un'occupazione RAM
fisica equivalente. Sono esclusi UI, acquisizione, server Codex e GPU.
Questi valori non dimostrano il consumo o la latenza dell'intera pipeline.
Il precedente contesto audio completo aveva richiesto 0,675 s sullo stesso
campione; il profilo 512 è stato scelto per segmenti brevi.

## Correzione qualità del 27 novembre

Per investigare errori di trascrizione su Discord sono stati generati tre
WAV italiani sintetici con Microsoft Elsa Desktop, senza riproduzione audio
o cattura. Sullo stesso testo di riferimento, 17 parole dopo normalizzazione
di maiuscole e punteggiatura, il vecchio Tiny/greedy/512 ha prodotto otto
operazioni di modifica di parola; Base/beam/full context con VAD ne ha
prodotte due. È un confronto su tre frasi TTS, non una misura di accuratezza
di una call reale o un WER rappresentativo dell'italiano.

Nelle prove dirette, silenzio e un click sintetico non hanno prodotto testo
con il nuovo VAD. Due ulteriori campioni sintetici, “sì” e “no”, sono stati
riconosciuti con la soglia vocale da 100 ms. Nessun audio di Discord è stato
ascoltato o salvato durante queste verifiche. Restano da collaudare rumore,
compressione, accenti e volume effettivi della call dell'utente.

Base richiede più memoria di Tiny: non va attribuito al nuovo profilo il
working set della tabella storica. Il modello viene comunque caricato solo
durante l'inferenza del segmento e rilasciato alla fine del processo.

Small e Turbo hanno ora un [confronto separato su italiano pubblico](review/iteration-13/TRANSCRIPTION.md). I valori storici della
notch inattiva descrivono la UI con i moduli di quel profilo: non includono
il processo Whisper, la VRAM usata da Vulkan o il server Codex condiviso.

Un harness Rust indipendente che include lo stesso `src/call_stt.rs` ha
superato sei test: linguaggio/percorso, cancellazione, sostituzione del
risultato, WAV e pulizia TEMP, parsing del file diagnostico e rifiuto dei
percorsi relativi/UNC. Il worker Rust reale ha poi riconosciuto la fixture
pubblica in 762 ms nell'ultima esecuzione, senza cattura, GUI o rete. Questa prova non sostituisce
la compilazione dell'app completa o il collaudo della pipeline della call.

Le prove del controller, dell'acquisizione WASAPI e della GUI sono riportate
separatamente nei record di validazione dell'iterazione.

## Fonti

- [CLI e opzioni della versione fissata](https://github.com/ggml-org/whisper.cpp/blob/v1.9.2/examples/cli/cli.cpp).
- [Licenza whisper.cpp](https://github.com/ggml-org/whisper.cpp/blob/v1.9.2/LICENSE).
- [Scheda e licenza MIT dei modelli convertiti](https://huggingface.co/ggerganov/whisper.cpp/blob/c521a4b02f422512d734391fdf08bb08c0862f68/README.md).
- [Licenza originale Whisper](https://github.com/openai/whisper/blob/main/LICENSE).
- [VAD supportato dal CLI della versione fissata](https://github.com/ggml-org/whisper.cpp/blob/v1.9.2/models/download-vad-model.sh).
- [Licenza Silero VAD](https://github.com/snakers4/silero-vad/blob/446da2ad95fc90d67a3b12e12ea3f5410b9b3ec4/LICENSE).
