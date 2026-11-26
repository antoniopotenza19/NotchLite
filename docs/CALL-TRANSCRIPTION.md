# Trascrizione locale delle call

Il modulo `call_stt` usa **whisper.cpp CPU** per convertire brevi segmenti
audio in testo sul PC. Non apre microfoni, sessioni WASAPI, browser o client
di rete. L'acquisizione dell'app selezionata e l'invio del testo a Codex sono
responsabilità distinte del controller della call.

## Preparazione

Da PowerShell, nella cartella del progetto o del pacchetto portatile:

```powershell
.\tools\Setup-CallTranscription.ps1
```

Lo script scarica dipendenze fissate e verifica SHA-256 **prima** di estrarre
o usare i file. Prepara `%LOCALAPPDATA%\NotchLite\whisper\runtime.json`;
non modifica PATH globale, avvio con Windows o impostazioni audio. Il setup
non avvia l'ascolto o un'inferenza. Richiede Windows x64 e una cartella locale
senza junction o altri reparse point nei componenti del percorso.

Il runtime è quello ufficiale [whisper.cpp v1.9.2](https://github.com/ggml-org/whisper.cpp/releases/tag/v1.9.2),
asset `whisper-bin-x64.zip`, circa 8,2 MB compressi. Vengono estratti soltanto
il CLI e le DLL CPU necessarie, con scelta dell'implementazione CPU effettuata
dal runtime. Il modello multilingue è `ggml-tiny-q5_1.bin`, 32.152.673 byte,
dal [repository originale dei modelli convertiti](https://huggingface.co/ggerganov/whisper.cpp/tree/c521a4b02f422512d734391fdf08bb08c0862f68).
I testi MIT di whisper.cpp e del modello Whisper sono conservati accanto al
runtime. Questi componenti opzionali restano processi/file separati, senza
collegamento della libreria Whisper nel processo della notch.

| File | SHA-256 |
| --- | --- |
| `whisper-bin-x64.zip` | `49dcc16de826f20bd53d44f947a1ae49dfa81f86cad67a64d80820cb192d674a` |
| `ggml-tiny-q5_1.bin` | `818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7` |
| `whisper-cli.exe` | `95e3c0b0e778ad9499eb0125f97c1dcf437dd9eb4ea77050b043574f93c2631d` |

Il pin del modello è `c521a4b02f422512d734391fdf08bb08c0862f68`.
Una nuova preparazione crea una directory runtime distinta; i vecchi file
non vengono cancellati implicitamente.

## Funzionamento e limiti

`SttWorker::new` verifica la configurazione e resta inerte: niente thread o
modello caricato. Il primo segmento avvia un worker; ogni segmento viene
elaborato da un solo processo CLI nascosto, che termina prima del successivo.
Il modello viene quindi rilasciato tra le inferenze. La configurazione iniziale
usa italiano, due thread CPU, GPU disabilitata, decoding greedy e contesto
audio 512, con segmenti PCM16 mono a 16 kHz lunghi al massimo otto secondi.

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
prova di accuratezza su italiano, accenti, rumore o più interlocutori.

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
