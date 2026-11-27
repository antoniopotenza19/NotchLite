# Mappa del cambiamento

Graphify AST locale aggiornato: 49 file, 2144 nodi, 5385 archi, zero token LLM.
Hash confrontati con i sorgenti compilati. Ponytail: cambiamento circoscritto
al backend ASR, conservando worker/segmentazione e routing AI esistenti.

- Setup prepara file fissati e manifest, senza eseguire il runtime.
- call_stt valida backend/device/thread e avvia un processo nascosto per segmento.
- Vulkan richiede no-flash e conferma metadati modello/backend/compute, drenando stderr senza salvarlo.
- call_session conserva il profilo congelato; assist lo mostra senza I/O nel rendering.
- call_coach mostra il modello ritornato dal server, senza cambiare provider o autorizzazioni.
- assist_layout lascia 62 DIP al profilo e allo stato AI; geometria pura testata.

La mappa non dimostra la qualità dell'audio, la sicurezza completa o la resa grafica.
