# Controller e pannelli call — revisione dell'iterazione 9

Revisione statica del 26 novembre 2026 su `assist.rs`, `assist_overlay.rs`,
`assist_layout.rs`, `call_session.rs` e `main.rs`, seguendo la skill locale
Ponytail Review e la checklist originale conservata nel progetto. Sono stati
letti i chiamanti e i percorsi di arresto di STT e ponte testuale, oltre ai
test. Carico previsto: un utente, una sessione confermata, una sola app audio
selezionata e un monitor secondario.

Il revisore ha scritto il componente `call_audio`; questa revisione è
indipendente rispetto al controller e ai pannelli, scritti dal coordinatore,
e non è una revisione indipendente della propria implementazione WASAPI.
Nessuna GUI, sessione audio, microfono o acquisizione reale è stata avviata.

## Rilievi corretti durante la revisione

1. **Revoca dell'audio prima dell'attesa dei worker.** `stop_call` attendeva
   l'arresto di IPC/STT prima di chiamare `monitor.stop_capture`. Durante tale
   attesa il flusso poteva continuare ad acquisire campioni. Ora la revoca del
   monitor è la prima operazione; la successiva pulizia non riabilita audio o
   code. La correzione è stata riletta nel controller.
2. **Arresto durante il cambio di collocazione.** Un cambio monitor/DPI con
   collocazione ancora valida nascondeva temporaneamente l'indicatore senza
   fermare la sessione. Ora `display_changed` ferma la sessione prima di
   nascondere/riposizionare e `sync` ferma anche al cambio di monitor scelto.
   Il catalogo può rimanere attivo; un nuovo ascolto richiede un altro click.
3. **Risposte vocali brevi perse.** La soglia di un secondo scartava una breve
   parola seguita da pausa. Ora una finestra con almeno 100 ms di voce viene
   completata con silenzio fino a un secondo. È presente una regressione per
   un breve riconoscimento; ciò non dimostra accuratezza ASR su voci reali.

È stata anche riletta la correzione del coordinatore per le ripetizioni:
la deduplicazione è ammessa soltanto per segmenti effettivamente sovrapposti
e risultati consecutivi. Una frase separata da pausa non viene eliminata
solo perché uguale alla precedente. Gli identificatori dei segmenti e la
tabella di sovrapposizione sono limitati e risultati saltati non autorizzano
deduplicazione attraverso una lacuna.

## Controlli sul codice corrente

- `Trascrivi` e `Trascrivi + AI` generano azioni diverse. Il primo ramo
  costruisce `CallSession` con `coach: None`: il controller non pubblica testo
  sul ponte in quella modalità. Il secondo crea esplicitamente il ponte
  locale; non avvia da solo una chat o un modello.
- Il rilevamento, `sync` e il timer non chiamano `confirm`. L'avvio passa
  dall'azione umana e dal candidato preciso. Un errore nella creazione della
  sessione dopo la conferma richiede subito lo stop del monitor.
- `capture: Some` forza visibilità e indicatore per Starting, Listening e
  Stopping, anche quando il catalogo candidati non contiene più la sessione.
  Selettore e contesto non sono modificabili durante queste fasi; Stop e
  chiusura restano disponibili.
- Lo stop nella coda delle azioni precede le altre azioni e rimuove conferme
  già accodate. Chiusura del pannello, disabilitazione, perdita del monitor,
  cambi di collocazione e Drop passano dalla revoca dell'ascolto. Non esiste
  ripresa automatica della cattura su una nuova identità.
- `tick_call` scarta una generazione diversa e termina la sessione quando
  l'audio è assente o Stopping. Stop svuota segmentazione e lavoro pendente;
  il testo già mostrato rimane disponibile nella UI, senza nuovo invio.
- Segmentazione, trascrizione visibile, tabella delle sovrapposizioni e testi
  del pannello hanno limiti espliciti. La coda STT sostituita e i risultati
  saltati producono un avviso di lacuna. Questi limiti non sono un benchmark
  RAM della GUI o del modello esterno.
- Il testo trascritto è trattato come testo. Il controller non lo passa a
  una shell o a un URI. Il deep link Codex è un'azione separata su click e
  valida un identificatore fisso. Le modalità diagnostiche sono anteriori a
  profilo, COM, mutex e finestre; non attivano l'acquisizione reale.

## Limiti delle evidenze

La revisione non esegue la GUI, l'acquisizione WASAPI, il modello vocale,
hotplug/DPI o conversazioni reali. I risultati di compilazione, test e prove
offline appartengono al record del coordinatore. Il ponte AI richiede un
assistente attivamente collegato: la sua presenza non garantisce un modello
in ascolto permanente dopo che quella chat ha terminato il lavoro.

Verdetto nel perimetro: rilievi sopra corretti, nessun altro blocco concreto
individuato nel controller. Resa e interazioni native richiedono ancora una
prova esplicita; questo documento non certifica l'intero prodotto.
