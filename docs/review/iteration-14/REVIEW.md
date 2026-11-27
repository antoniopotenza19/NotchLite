# Revisione indipendente — lettura compatta, vetro nativo e diagnostica Teams

Stato: revisione del freeze 0.1.13 completata; nessun rilievo concreto
aperto. La revisione applica Ponytail Review del repository. Il revisore
non esegue cargo, binari, inferenze, audio, desktop o screenshot. Distingue
la lettura indipendente dai test eseguiti dall'integratore.

## Cosa cambia

La vista call separa lettura e controlli: trascrizione, risposta e Stop
restano nella prima, sorgente, contesto e consenso esplicito nella seconda.
La cornice Graphite richiede un materiale DWM opzionale, con testo su fondo
opaco. Il catalogo audio aggiorna il proprietario della sessione e può
proporre un indizio Teams senza un nuovo evento duck. Contatori limitati
distinguono pacchetti mancanti, PCM silenzioso, soglia vocale, segmenti ed
esiti STT; non archiviano audio.

## Rilievo diagnostico corretto

1. **Un errore di avvio del monitor non compare nel reader**
   (`src/assist.rs`, `manage_workers` e `call_view`). Se `AudioMonitor::start`
   fallisce, il messaggio finisce in `self.note`, usato dalla dashboard.
   Aprendo manualmente la call con dashboard spenta, il reader mostra
   attesa/nessuna trascrizione e il consenso è disabilitato senza spiegare
   la causa. È un percorso preesistente, rilevante per la diagnostica ora
   modificata; non è una riproduzione del problema Teams dell'utente.
   L'integratore ha aggiunto `monitor_error`: il ramo reale di startup lo
   assegna, successo/disabilitazione lo puliscono e `call_view` lo mostra
   nello status e nella trascrizione vuota. Il nuovo test puro
   `manual_reader_explains_monitor_start_failure_with_dashboard_disabled`
   copre lettura dell'errore e consenso disabilitato. La correzione è stata
   riletta indipendentemente; rilievo chiuso.

## Confini letti e ipotesi non confermate

- I comandi start/selezione della form nascosta sono respinti. Il cambio
  a lettura segue il cambio del capture, non il solo click in coda. Stop
  resta disponibile nelle due viste; chiusura e distruzione revocano la
  sessione. Il focus passa a Stop dopo l'avvio se la finestra è attiva.
- Le due geometrie hanno budget separati, testati a 300×480 DIP e DPI
  multipli. I controlli nativi vengono nascosti, senza ricreare gli editor;
  il testo e il percorso di scroll esistente restano in uso. Questo non è
  una prova di tastiera o wrapping nativo.
- DWM è richiesto solo per Graphite con build supportata, composizione,
  trasparenza, contrasto e risparmio energia conosciuti. Un rifiuto ripristina
  materiale assente e margini zero. Le richieste avvengono fuori dal paint;
  non aggiungono worker o timer. Il paint espone una cornice limitata e
  mantiene opachi corpo/editor. Il contrasto elevato usa colori di sistema.
- Il raw HRESULT di `GetProcessId` distingue l'owner singolo da quello
  ambiguo. Owner/PID cambiato o processo morto non eredita il consenso;
  registrazione precedente e coda vengono ritirate, senza seguire un sibling
  o sostituire il PID con il root Teams.
- L'indizio Teams deriva da nomi esatti e antenati vivi con tempi di
  creazione coerenti, massimo otto passaggi e snapshot limitato. Non deriva
  dall'etichetta visualizzata. Un suono Teams può proporre un candidato:
  l'indizio non certifica una call e non avvia ascolto.
- La salute audio osserva numeri e istanti, senza conservare campioni nel
  watcher. Dopo dodici secondi distingue assenza di pacchetti e silenzio;
  la ripresa rimuove l'avviso. Nessuno dei due stati interrompe da solo una
  call quieta. I contatori del Segmenter non ricontano l'overlap trattenuto.

## Fonti primarie verificate

[GetProcessId](https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-getprocessid)
documenta sessioni su più processi e aggiornamento dello stato al cambio
stream. Il codice conserva il vincolo sull'identità invece di usare il
primo PID ambiguo.

[DWM_SYSTEMBACKDROP_TYPE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type)
richiede Windows 11 build22621 e definisce il materiale transient.
[DwmExtendFrameIntoClientArea](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmextendframeintoclientarea)
richiede riapplicazione al cambio composizione, gestita dal nuovo ramo.
L'accettazione dell'API non dimostra la resa del vetro.

## Limiti obbligatori

Non è stata ascoltata o riprodotta una call Teams. La presenza del pannello
non dimostra conferma, PCM non silenzioso o un segmento sopra soglia; il
nuovo codice rende distinguibili questi passaggi, senza stabilire la causa
dell'errore dell'utente. Restano da provare call reale, proprietà del renderer
Windows in uno stream switch, tastiera, contrasto e vetro renderizzato,
condivisione schermo e risorse durante una sessione lunga.

## Evidenze lette dell'integratore

La suite finale dopo la correzione monitor riporta 174 test superati, zero
falliti, 0,90 s; GNU release completata in 1 min 19 s. Le 52 impronte degli input di
build sono state ricalcolate dal revisore: nessuna differenza dal record.
Anche le 50 impronte della mappa aggiornata corrispondono ai sorgenti.
La mappa finale riporta 2.198 nodi e 5.524 archi; è una mappa statica, non una
verifica del comportamento Windows.

Il diagnostico metadati del binario ha osservato una sessione render, zero
sessioni Teams e nessun errore. Dichiara cattura, microfono e GUI spenti;
non conferma l'identificazione di una call Teams. Il file pubblico di sei
secondi è stato trascritto in 5.262 ms con `gpuVerified=true`, senza GUI o
acquisizione: prova il worker integrato, non l'audio dell'utente.
Il revisore ha ricalcolato il digest del binario compilato e della copia
portatile: entrambi sono
`894ad31219db0fcf20dec7654b12b18036e2c1a87f39ee749fa342843aa1f6b8`.
Nessuno di questi comandi è stato eseguito dal revisore.

Il revisore ha letto `native-metadata.json` dell'integratore: editor della
lettura visibili, form nascosta, pulsanti Stop e Controlli visibili, nessuna
sovrapposizione con la notch. Ascolto falso e Stop disabilitato sono coerenti
con la sessione inattiva. Il readback DWM restituisce valore3 e HRESULT0:
conferma la richiesta transient, non l'aspetto del vetro. Nessun input sul
desktop o screenshot è dichiarato per tale controllo.

Il campione inattivo dell'integratore contiene 20 rilevazioni in 22,926 s:
working set massimo campionato 77,469 MiB, commit privato massimo campionato
90,902 MiB e CPU totale media0,23%. Descrive il processo installato con
reader aperto e ascolto spento. Non include VRAM o altri processi, non misura
ASR/AI durante una call e non dimostra una riduzione rispetto a profili
precedenti differenti. Il revisore ha letto JSON e aggregati, senza
eseguire il campionatore.

Verdetto: pubblicabile come anteprima sperimentale nel perimetro verificato,
con il rilievo diagnostico corretto e i limiti nativi/live-call espliciti.

## Snapshot sorgente letto

| File | SHA-256 |
| --- | --- |
| `src/assist.rs` | `6c30d1ca4a9842e97df78b5c2605536b36123fd671817ddc728dcabb3be34d92` |
| `src/assist_overlay.rs` | `81ccbd6ed8684d659b73ab98bd1539fee53c1cf827a1a675fc6abee5048aafde` |
| `src/assist_layout.rs` | `59da25359a5799fdead4fd7a5a45551fe898c089d10870bc9df98e74a0ac55f2` |
| `src/assist_glass.rs` | `dc575035623302aeb110346db90ba2f9084ab825ba1212a18ef5e15339ef3d16` |
| `src/call_audio.rs` | `1af84f7376e4f03a8084f0d16ccc4213e43aedd921b1aeb3718d1ef4101dac9d` |
| `src/call_audio/windows.rs` | `1c18d3fa1c2595972e5b7a3987187ab5da1c70ebfec1959331480e4834771bc3` |
| `src/call_audio/capture.rs` | `d0b4ff8a9c0d41ece9bf4195f199f7ed9707d0143e8d98dbde0cb4bf633ff6f6` |
| `src/call_session.rs` | `125e1f846e7d562fce83c636e9d87b30e0d1518cdeca6dd894d0f7a26ede2d93` |
| `src/call_coach.rs` | `c32cba3acba8675901e4b33c17f80c02c7c529cc9ca22b3d2cc04a52c93934bb` |
| `src/main.rs` | `da29a1319b1d1454b5a6e8bc8a6722d40e912a48a547f57b98b5ad56622d9077` |
| `Cargo.toml` | `81f3b7551f88d0a13afd3e67687d0c3c22160e8d21c507a20264d3e8e5baec82` |
| `Cargo.lock` | `7af8724a9e640e8eb474a1e0f75b5e38b0a6fbd55a00c3bee2e5e2a31af23b9e` |
