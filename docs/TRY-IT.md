# Provare NotchLite

1. Scarica NotchLite-portatile.zip dalla sezione Releases del repository.
2. Estrai l'intera cartella.
3. Apri notchlite.exe con un doppio clic. Non serve installare Rust.
4. Il pannello si apre in alto al centro del monitor principale ed è presente in taskbar/Alt-Tab. Le frecce cambiano pagina.
5. Escape riduce il pannello alla capsula. Se stai scrivendo, il primo Escape annulla l'editor. La capsula si apre al passaggio del mouse; rilanciare l'EXE o usare Open panel dal menu la apre in modo persistente.

Il profilo iniziale attiva deposito file, attività e timer focus. Settings è l'ultima pagina e permette di abilitare gli altri moduli già presenti. Le funzioni avanzate della roadmap restano da realizzare. L'interfaccia ereditata è attualmente in inglese.

Per uscire, fai clic destro sulla notch o sull'icona nell'area di notifica, quindi scegli **Exit**. Se l'icona è nascosta, apri le icone nascoste accanto all'orologio. Il menu sulla notch è nuovo nella 0.1.2 e resta da collaudare.

La modalità --background avvia soltanto la capsula, che si nasconde quando Windows segnala fullscreen/presentazione. Il pannello aperto esplicitamente resta visibile finché viene ridotto; la sospensione grafica riprende il normale comportamento della capsula.

I dati utente restano in %APPDATA%/NotchLite e %LOCALAPPDATA%/NotchLite; il pacchetto non abilita automaticamente l'avvio con Windows. La cartella dell'eseguibile è portabile, i dati seguono queste cartelle Windows.
