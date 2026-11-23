# Team di sviluppo NotchLite

Questo team usa Graphify per orientarsi nel codice e Ponytail per fare modifiche complete e contenute. Sono strumenti di sviluppo: non vengono inclusi nel processo dell'app e non incidono sulla sua RAM.

## Attivazione proporzionata

Il coordinatore parte dall'incarico dell'utente. Per una correzione piccola può implementare e verificare direttamente. Per cambiamenti a più moduli, concorrenza, persistenza o confini di input, assegna i ruoli necessari; non serve attivarli tutti. Quando servono pareri indipendenti, il revisore riceve sorgenti, obiettivo e prove, senza un verdetto desiderato.

Il file [team.json](team.json) conserva le responsabilità. I ruoli possono essere svolti dallo stesso agente nei compiti piccoli; in quel caso la verifica è dichiarata come autocontrollo, non indipendente. Il numero di agenti segue i limiti del sistema ospite.

## Ruoli e passaggi

| Ruolo | Responsabilità e output | Limiti di scrittura |
|---|---|---|
| Coordinatore | Definisce obiettivo, file assegnati e criteri osservabili; integra le risposte e risolve conflitti | Solo quanto autorizzato dall'incarico |
| Mappa | Traccia moduli, chiamanti, stato condiviso e superfici di input; cita file/righe e hash | Grafo corrente e note assegnate; non modifica il prodotto |
| Implementazione | Corregge la causa nel punto condiviso più piccolo; aggiorna i chiamanti e un controllo significativo | File e test assegnati esplicitamente |
| Revisione | Usa Ponytail Review; cerca un caso concreto e tenta di confutarlo; distingue bug da ipotesi | Report assegnato o risposta in chat; non modifica il codice dell'autore |
| Validazione | Compila ed esegue le prove consentite; separa test puri, integrazione, UI e benchmark | Fixture sintetiche, output di prova e report assegnati |

Prima di delegare, dichiarare ownership dei file e che il workspace è condiviso. Non annullare gli edit altrui. Una revisione di sicurezza completa segue la skill di sicurezza disponibile nel sistema e le sue regole sulle riproduzioni; un normale controllo qualità non abilita scanner o prove su servizi reali.

## Graphify senza installazioni implicite

Il wrapper [Update-CodeGraph.ps1](../tools/Update-CodeGraph.ps1) usa le API AST locali di una distribuzione Graphify già presente. Non usa backend LLM, credenziali o rete e non avvia il codice Rust.

```powershell
.\tools\Update-CodeGraph.ps1
# Se Graphify è in un interprete diverso da python:
.\tools\Update-CodeGraph.ps1 -GraphifyPython .\.venv\Scripts\python.exe
```

Il secondo comando è un esempio: indicare l'interprete che contiene davvero il pacchetto. Lo script non crea un ambiente virtuale e non installa nulla. Se manca Graphify, leggere sorgenti e chiamanti direttamente oppure installarlo solo in un incarico che lo richieda.

L'output predefinito è graphify-out/: graph.json, GRAPH_REPORT.md e SOURCE_SNAPSHOT.json. La snapshot contiene hash relativi al repository; rigenerare la mappa dopo modifiche strutturali o quando gli hash non corrispondono. Il grafo AST può omettere dynamic dispatch e collegamenti COM/FFI; non dimostra raggiungibilità runtime né sicurezza.

## Ponytail e revisione

La [skill di sviluppo](../.agents/skills/notchlite-development/SKILL.md) applica Ponytail al solo progetto. La [skill di review](../.agents/skills/ponytail-review/SKILL.md) applica la checklist alla modifica richiesta. Le fonti MIT originali, non eseguibili, sono conservate accanto alle skill con autore e revisione.

La modifica minima deve risolvere l'intero incarico. Riutilizzare prima gli helper del progetto e le API native; non tagliare validazione, accessibilità o gestione errori per accorciare il diff. Un problema di codice richiede input o situazione concreta, effetto e riferimento. Non cambiare il codice per soddisfare una preferenza stilistica del revisore.

La validazione registra cosa è passato e cosa non è stato verificato. Le prove mancanti non diventano automaticamente nuove richieste di approvazione: continuare il lavoro consentito, segnalando i limiti. Pubblicazioni e installazioni seguono l'autorizzazione già data dall'utente e il perimetro corrente.
