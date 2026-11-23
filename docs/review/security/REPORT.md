# Revisione di sicurezza statica — NotchLite

Revisore: security_audit. Data: 23 novembre 2026. Perimetro assegnato: ingressi OLE/file, storage, rete, worker e integrazioni media/meteo/notifiche/calcio/Claude. Il coordinatore ha autorizzato le sole correzioni minime ai file droptarget.rs e weather.rs.

## Risultato

- **S1:** gap confermato nel filtro dei link web, corretto nel sorgente. Prima il formato OLE UniformResourceLocatorW evitava il filtro http(s), e il writer rimuoveva i controlli invece di rifiutarli. Non è dimostrata RCE; lo scenario di attivazione di handler Windows resta non provato.
- **S2 (P2):** refresh meteo successivo alla disattivazione, confermato staticamente e corroborato indipendentemente dal revisore qualità. Corretto con stop channel e attesa interrompibile. Una richiesta già in corso può completarsi.
- **S3:** limiti applicativi di buffering HTTP, timeout e immagini da verificare; nessuna severità assegnata a uno sfruttamento non dimostrato.
- Ipotesi di command injection nella shelf e path traversal via team ID sono state confutate dai guard/sink presenti.

Il difetto di uscita individuato dal revisore qualità è stato verificato indipendentemente: DestroyWindow invia WM_DESTROY mentre il RefCell è ancora preso in prestito; il ramo rientrante saltava l'unico PostQuitMessage. La correzione appartiene al revisore qualità e non viene contata come una seconda vulnerabilità.

## Prove e limiti

Nessun exploit, handler, endpoint live o credential è stato provato. Manca un sandbox OS con rete disabilitata e target/toolchain in sola lettura: le ipotesi di sicurezza sono state analizzate sui sorgenti. I nuovi test sono regressioni normali con URL sintetici e canali/fetch sostitutivi; l'esecuzione della suite è lasciata al coordinatore.

I record findings.json conservano trigger, traccia, controevidenza, assunzioni, correzioni e chiusure. coverage.json distingue superfici ispezionate, parziali, non ispezionate e bloccate. source-identifiers-before.json identifica i file prima delle correzioni di questo revisore; source-identifiers-after.json identifica i due file corretti. Questi identificatori non equivalgono a uno snapshot atomico dell'intero lavoro del team.

Fonti primarie per la verifica indipendente di Exit: [DestroyWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-destroywindow) e [GetMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew).

Questa revisione non certifica l'intero prodotto come sicuro; GUI, codec nativi, dipendenze, carichi elevati e comportamento di rete richiedono ulteriori verifiche esplicite.
