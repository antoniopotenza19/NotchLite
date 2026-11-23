# Nota dopo le correzioni

La mappa graph.json/html, GRAPH_REPORT.md, METADATA.json, SOURCE_SNAPSHOT.json e ARCHITECTURE_REVIEW.md in questa cartella descrivono lo **snapshot prima delle correzioni dell'audit**. Sono conservati come evidenza storica; non sono un grafo aggiornato del codice finale.

Dopo la revisione sono stati modificati anche droptarget.rs e pages/weather.rs. Lo stato successivo ha 24 file Rust: 18 ereditati identici, 5 ereditati modificati (main, shell, store, droptarget, weather) e 1 nuovo (profile). La descrizione corretta resta versione derivata da Ada, non riscrittura completa.

La suite attuale contiene 15 test in cinque file: 7 calcio, 3 URL/drop, 1 profilo, 2 shell/Unicode, 2 worker meteo. Il coordinatore ha riportato 15 test passati nel normale percorso di build con input sintetici e fetch/canali simulati. Questo non costituisce esecuzione di exploit né prova della UI. Il revisore indipendente non ha rieseguito i test.

La verifica conclusiva delle patch e i limiti rimasti sono in ../FINAL_REVIEW.md. I relativi hash individuano i tre file centrali sottoposti al secondo giro. Il grafo storico resta utile per orientarsi, ma numeri, righe e hash devono essere riferiti allo snapshot indicato prima di usarli come evidenza.
