# Revisione indipendente — profili STT e backend CPU/Vulkan

Stato: lettura indipendente del freeze sorgente 0.1.12 e delle evidenze STT
completata. Nessun difetto concreto aperto nel perimetro letto.

## Perimetro e metodo

La revisione segue Ponytail Review: prima i chiamanti e i confini, poi il
codice e le prove del freeze. Riguarda manifest del profilo, subprocess STT,
runtime opzionali, setup con hash e licenze e indicazioni visibili del
profilo e del modello AI. Non richiede nuove chiamate a modelli online.
Il revisore non avvia audio, microfono, riproduzione, GUI o desktop e non
esegue inferenze. Le misure degli altri autori saranno distinte dalla
lettura indipendente delle loro evidenze.

## Provenienza del runtime candidato

Il runtime Vulkan è distribuito da SubtitleEdit/support-files; non è un
binario ufficiale ggml-org. Il setup seleziona la
[release whispercpp-195](https://github.com/SubtitleEdit/support-files/releases/tag/whispercpp-195),
il cui ZIP di 12.964.889 byte ha SHA-256
`89f2c36455939ad42e08b2107bd6b47105b6d9c1a2b0916cbb500ec27deb520a`:
il revisore ha ricalcolato il digest del file locale e confrontato i
metadati della release. Il pin del CLI è
`1ef8f16edf2f8df429e7e1d18c9af073f66d594813e9aae739e5bdf015a61985`.
Il [workflow pubblico fissato](https://github.com/SubtitleEdit/support-files/blob/358047ebca4410be9636974795e68e01427b6f53/.github/workflows/build-whispercpp-release.yml)
compila il target Vulkan dall'upstream indicato e separa i suoi artefatti.
Questa è tracciabilità di distribuzione e build, non una riproduzione
indipendente del binario né un'attestazione crittografica sorgente/binario.

Il setup congelato mantiene digest, set di DLL separati, controlli di
percorso locale/reparse point, rifiuto delle entry ZIP pericolose e tre
licenze MIT separate. Estrae solo i nomi nativi consentiti; non esegue il
binario scaricato. Il VAD incluso nel bundle non sostituisce quello già
fissato dal progetto. La scelta del runtime non richiede un'installazione
globale del Vulkan SDK. Su questa lettura statica non emerge un difetto
concreto del setup; la sua esecuzione finale compete all'integratore.

## Confini verificati nel codice

Il [backend upstream fissato](https://github.com/ggml-org/whisper.cpp/blob/d1be6fde11ac6e0407606b4e42fe72d34add8037/src/whisper.cpp)
può non inizializzare una GPU e aggiunge comunque un backend CPU; il VAD
del pin usa CPU. Perciò una preferenza «Vulkan» e l'enumerazione dei device
non provano l'offload. Lo stato deve distinguere configurazione richiesta
da backend osservato, oppure indicare esplicitamente che è una richiesta.

`GGML_VK_VISIBLE_DEVICES` ereditato può rimappare gli ordinali. La selezione
e l'ambiente del solo processo figlio devono essere coerenti con il device
scelto, senza modificare l'ambiente globale. Un fallback implicito non deve
essere presentato come GPU effettiva o nascondere tempi CPU incompatibili
con il limite del processo.

Nel patchset letto, i manifest precedenti mantengono CPU/2 thread; backend,
thread e device sono valori chiusi e limitati, non stringhe CLI. Il processo
figlio rimuove gli override di Whisper/Vulkan ereditati e fissa la directory
dei backend senza cambiare l'ambiente del sistema. Vulkan disattiva la flash
attention e osserva solo i metadati di inizializzazione ASR: allocazione del
modello sul device richiesto, backend e buffer di calcolo. Il parser congela
l'esito prima dei log VAD su CPU. Una sola enumerazione non basta. Un testo
non vuoto senza questa conferma viene rifiutato; un errore GPU interrompe il
figlio. Lo zero digitale continua a non avviare alcun processo.

La pipe viene drenata anche dopo l'esito, con memoria limitata a una riga di
1.024 byte; non viene salvato un log della call. Stop, timeout da 30 secondi,
coda limitata e controllo della generazione restano sul percorso esistente.
La label del profilo deriva dalla configurazione della sessione e distingue
Vulkan richiesto; il nome AI viene dalla risposta effettiva di avvio del
thread, senza fissare un nuovo modello. Nessuna inferenza è introdotta nel
rendering. Il diagnostico del binario integrato è descritto sotto.

## Qualità e prestazioni

Il benchmark iniziale seleziona le prime 12 righe FLEURS italiane di durata
4–8 secondi prima del riconoscimento, senza tagliare l'audio rispetto al
riferimento. I conteggi delle modifiche alle parole usano la stessa
normalizzazione e la stessa distanza di edit per i profili confrontati.
Le serie parziali di due clip non devono essere confrontate con i totali
di dodici come se avessero lo stesso campione.

Il revisore ha letto i risultati e verificato tutte le dieci impronte delle
evidenze elencate in `work/iteration-13/stt/final-evidence.json`. Il rapporto
STT congelato ha SHA-256
`e372eadcccaff8fb2bbc6c48f23de29adcc2c539622df67f7945cf10f059fff9`.
Le esecuzioni sono del ricercatore, non del revisore.

| Profilo sullo stesso campione di 159 parole | Modifiche alle parole | Mediana processo | Massimo osservato |
| --- | ---: | ---: | ---: |
| Base Q5_1 CPU2, runtime 1.9.2 | 39 | 1,672 s | 1,775 s |
| Small Q5_1 CPU4, runtime 1.9.2 | 27 | 3,109 s | 13,303 s |
| Turbo Q5_0 Vulkan4, vendor195 senza flash attention, nuova prova autorizzata | 6 | 1,134 s | 1,165 s |

La nuova prova195 comprende 29 processi con exit0: dodici clip naturali,
tre degradate Opus24/-18dB, sì/no sintetici, silenzio/click, nove segmenti
prodotti dal vero Segmenter Rust e un diagnostico con log/contatori GPU.
Il replay del Segmenter e della funzione merge produce zero modifiche su
41 parole sia per i tre originali sia per le tre versioni degradate. Il
log autorizzato conferma modello Vulkan0, backend e buffer, distinti dal
successivo VAD su CPU. La prova contatori registra 1.181,39 MiB di memoria
GPU dedicata campionata e 1,5 MiB condivisa; sono picchi di categorie
separate. Non si deduce la VRAM dal commit privato CPU.

Quella serie era già calda: il primo run195 preliminare aveva impiegato
3,658 s e usato più memoria. Il precedente candidato192 aveva avuto anche
un crash isolato documentato; i run riusciti successivi non ne provano la
causa. Ventinove successi195 non dimostrano stabilità indefinita. Turbo CPU
aveva superato 30 s in una prova, oltre il timeout del prodotto: non è un
fallback live silenzioso accettabile. Nessun cambio cieco della soglia RMS
è giustificato dai tre replay Base, che aggregati non peggiorano rispetto
ai file interi.

Working set del figlio, memoria privata impegnata, RAM della UI e memoria
GPU sono misure diverse. Il campione non misura la RAM totale dell'app;
il working set ridotto di una serie calda non è un budget generale.
FLEURS è parlato letto e questo campione comprende undici voci classificate
femminili e una maschile. I risultati non provano accuratezza su Discord,
colloquio spontaneo, accenti, sovrapposizione o audio reale dell'utente.
Il replay copre Segmenter/merge; non copre acquisizione o code in una call.

## Validazione e limiti della revisione

Sono stati letti il diff completo, la scoperta/configurazione e lo spawn
STT, i chiamanti di `Configuration`, `CallSession`, il percorso di Stop,
la composizione della label e l'origine di `snapshot.model`. I nuovi test
coprono parsing/range/compatibilità, override dell'ambiente e receipt GPU
che distingue enumerazione/VAD. Il codice AI di generazione resta invariato;
nessuna chiamata online è necessaria per leggere il nuovo nome modello.
Il flag diagnostico `gpuVerified` può essere vero soltanto per testo
Vulkan non vuoto accettato dal percorso che impone la receipt.

Il revisore ha letto i log della suite dell'integratore: 163 test superati,
zero falliti, 0,91 s; build GNU release completata in 31,59 s. Le 51 impronte
degli input di build corrispondono ancora ai file presenti. Anche le 49
impronte della mappa Graphify coincidono; la mappa aggiornata riporta 2.144
nodi e 5.385 archi. Questa lettura non equivale a rieseguire i test.

`product-file-checks.json` registra sei controlli del CLI integrato, senza
GUI o acquisizione: frase pubblica, frase con Opus/volume ridotto e sì/no
sintetici producono testo e `gpuVerified=true`; silenzio e click non
producono parole. Il silenzio digitale termina in 20 ms, senza inferenza,
coerentemente con `gpuVerified=false`. I tempi delle quattro prove vocali
sono 3.462, 1.530, 1.087 e 1.067 ms; nessuno è un tempo audio→pannello.
Il revisore ha ricalcolato il digest del binario compilato e della copia
portatile: entrambi coincidono con il record, SHA-256
`57302cea862737f4d900390c3781dfd604a12d28741e711654bd851a0b495ebf`.

Il revisore non ha eseguito cargo, binari, setup, inferenze o prove native.
Non sono verificati resa grafica delle label alle larghezze minime, uso
del mouse, cattura reale, qualità di una call o stabilità GPU lunga. In
particolare il rischio di wrapping della nuova label è stato segnalato:
l'integratore ha aumentato il blocco provider da 42 a 62 DIP, conservando
le prove geometriche a 300×480 DIP e DPI multipli. La leggibilità dei testi
variabili richiede ancora collaudo visivo nelle dimensioni strette; non è
stata prodotta una schermata o riprodotto un difetto grafico.

Verdetto: pubblicabile come anteprima sperimentale nel perimetro verificato.
I risultati sostengono Turbo/Vulkan195 senza flash attention come candidato
misurato su questo PC; Small/CPU resta un'alternativa esplicita. Non è una
certificazione della qualità o della stabilità durante una call reale.

## Snapshot sorgente letto

| File | SHA-256 |
| --- | --- |
| `src/call_stt.rs` | `825321008509ff9a58725011b4ddf0667309a68767eb10c0f85bc0d25b5d21fc` |
| `src/call_session.rs` | `57e33d28d686418cfa114017907905d5df79077cbb195c463b972ea0c28f023a` |
| `src/call_coach.rs` | `c59f3c4b3e129e4f5c9e1b65417816a5683764d14792c0ead04db35324cf24cc` |
| `src/assist.rs` | `007781792036f42c5ed0c57b0ebd34df77d793e944781abcf481f50688dfbc5b` |
| `src/assist_layout.rs` | `74dbbf89ef5a247d2556b66ec3a4bf1b78facb63b9807c38f3bc4b1c9bd56bc8` |
| `src/main.rs` | `5bebc1a6250839c7cae7f46c62cb625efcf933436d399fa33a1b49d9a842edd0` |
| `Cargo.toml` | `c7a0f66bb7f561367d3c76ef28974a01b3725fc8cb02fc40d138adb7ac30002e` |
| `Cargo.lock` | `60794f82fff20b0df7cb2b4fb4800e3786425e0f72a5738515844a9c8c2589a8` |
| `tools/Setup-CallTranscription.ps1` | `900696226690cff560760f5754167d01970488b8b10d21bf0ea22d9c0ae68b3d` |
| `docs/CALL-TRANSCRIPTION.md` | `4fce6f601cdb4783572ccff8da028b3d1c83292323b99dafc6c8eeca83ded21b` |
