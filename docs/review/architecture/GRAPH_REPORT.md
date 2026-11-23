# Limiti di questa mappa

Snapshot AST prima delle correzioni dell'audit. Questo grafo non è stato rigenerato dopo le patch; vedere POST_PATCH_NOTE.md per lo stato successivo.

Estrazione AST locale dei soli sorgenti Rust; nessun servizio esterno e nessuna esecuzione del codice applicativo. Le relazioni estratte non dimostrano raggiungibilità runtime né correttezza dei binding Win32. Il clustering separa soprattutto file/moduli e non equivale a isolamento a runtime. Gli eventuali collegamenti tra omonimi devono essere verificati sul sorgente. Le costanti senza entità AST possono essere sottorappresentate. Costo aggiuntivo LLM della pipeline Graphify: 0 token; la revisione umana/assistita di accompagnamento non è conteggiata da questo strumento.

# Graph Report - src  (2026-11-23)

## Corpus Check
- Corpus is ~34,946 words - fits in a single context window. You may not need a graph.

## Summary
- 853 nodes · 1983 edges · 22 communities (21 shown, 1 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 13 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Risultati sportivi|Risultati sportivi]]
- [[_COMMUNITY_Finestra e interazioni|Finestra e interazioni]]
- [[_COMMUNITY_Grafica Direct2D|Grafica Direct2D]]
- [[_COMMUNITY_Media e testi|Media e testi]]
- [[_COMMUNITY_Deposito e trascinamento|Deposito e trascinamento]]
- [[_COMMUNITY_Calendario|Calendario]]
- [[_COMMUNITY_Ingresso drag-and-drop|Ingresso drag-and-drop]]
- [[_COMMUNITY_Timer focus|Timer focus]]
- [[_COMMUNITY_Notifiche Windows|Notifiche Windows]]
- [[_COMMUNITY_Lista attività|Lista attività]]
- [[_COMMUNITY_Contratto delle pagine|Contratto delle pagine]]
- [[_COMMUNITY_Meteo|Meteo]]
- [[_COMMUNITY_Utilizzo Claude|Utilizzo Claude]]
- [[_COMMUNITY_Impostazioni|Impostazioni]]
- [[_COMMUNITY_Statistiche del sistema|Statistiche del sistema]]
- [[_COMMUNITY_Mascotte|Mascotte]]
- [[_COMMUNITY_Worker e messaggi|Worker e messaggi]]
- [[_COMMUNITY_Persistenza locale|Persistenza locale]]
- [[_COMMUNITY_Client HTTP|Client HTTP]]
- [[_COMMUNITY_Animazioni|Animazioni]]
- [[_COMMUNITY_Avvio applicazione|Avvio applicazione]]
- [[_COMMUNITY_Profilo iniziale|Profilo iniziale]]

## God Nodes (most connected - your core abstractions)
1. `App` - 61 edges
2. `MediaPage` - 32 edges
3. `ShelfPage` - 31 edges
4. `CalendarPage` - 30 edges
5. `wndproc()` - 28 edges
6. `Event` - 27 edges
7. `TodoPage` - 27 edges
8. `DrawCtx` - 25 edges
9. `Option` - 22 edges
10. `Color` - 21 edges

## Surprising Connections (you probably didn't know these)
- `with_app()` --calls--> `F`  [INFERRED]
  src/shell.rs → src/worker.rs
- `read_files()` --calls--> `HDROP`  [INFERRED]
  src/droptarget.rs → src/shell.rs
- `send()` --calls--> `LPARAM`  [INFERRED]
  src/worker.rs → src/shell.rs
- `send()` --calls--> `WPARAM`  [INFERRED]
  src/worker.rs → src/shell.rs

## Import Cycles
- 1-file cycle: `src/droptarget.rs -> src/droptarget.rs`
- 1-file cycle: `src/pages/shelf.rs -> src/pages/shelf.rs`
- 1-file cycle: `src/gfx.rs -> src/gfx.rs`
- 1-file cycle: `src/shell.rs -> src/shell.rs`
- 1-file cycle: `src/mascot.rs -> src/mascot.rs`
- 1-file cycle: `src/pages/calendar.rs -> src/pages/calendar.rs`
- 1-file cycle: `src/pages/claude.rs -> src/pages/claude.rs`
- 1-file cycle: `src/pages/football.rs -> src/pages/football.rs`
- 1-file cycle: `src/pages/media.rs -> src/pages/media.rs`
- 1-file cycle: `src/worker.rs -> src/worker.rs`
- 1-file cycle: `src/pages/mod.rs -> src/pages/mod.rs`
- 1-file cycle: `src/pages/settings.rs -> src/pages/settings.rs`
- 1-file cycle: `src/pages/system.rs -> src/pages/system.rs`
- 1-file cycle: `src/pages/todo.rs -> src/pages/todo.rs`
- 1-file cycle: `src/pages/weather.rs -> src/pages/weather.rs`
- 1-file cycle: `src/profile.rs -> src/profile.rs`
- 1-file cycle: `src/store.rs -> src/store.rs`

## Communities (22 total, 1 thin omitted)

### Community 0 - "Risultati sportivi"
Cohesion: 0.07
Nodes (74): apply_summary(), badge_colors(), Change, changes(), crest_key(), Crests, days_to_ymd(), detects_goals_kickoff_and_full_time() (+66 more)

### Community 1 - "Finestra e interazioni"
Cohesion: 0.06
Nodes (42): DropData, DropHandler, HDROP, HICON, LPARAM, LRESULT, Mascot, add_tray_icon() (+34 more)

### Community 2 - "Grafica Direct2D"
Cohesion: 0.06
Nodes (33): D2D1_COLOR_F, D2D_RECT_F, ID2D1Bitmap, ID2D1DeviceContext, ID2D1Factory1, ID2D1GradientStopCollection, ID2D1SolidColorBrush, ID2D1StrokeStyle (+25 more)

### Community 3 - "Media e testi"
Cohesion: 0.07
Nodes (50): AtomicU64, Manager, MediaProps, Cmd, ease_out_cubic(), fetch_lyrics(), fmt_time(), Hit (+42 more)

### Community 4 - "Deposito e trascinamento"
Cohesion: 0.07
Nodes (38): AtomicU32, BOOL, GUID, HRESULT, IDropSource, IDropSource_Vtbl, drag_out(), DropSource (+30 more)

### Community 5 - "Calendario"
Cohesion: 0.09
Nodes (22): CalendarPage, Date, days_in_month(), Event, Hit, is_leap(), Layout, local_today() (+14 more)

### Community 6 - "Ingresso drag-and-drop"
Cohesion: 0.10
Nodes (36): Cell, FORMATETC, IDataObject, IDropTarget, IDropTarget_Impl, Kind, POINTL, as_web_url() (+28 more)

### Community 7 - "Timer focus"
Cohesion: 0.11
Nodes (20): Duration, Btn, FocusPage, format_mm_ss(), Layout, LocalDate, Phase, State (+12 more)

### Community 8 - "Notifiche Windows"
Cohesion: 0.11
Nodes (28): EventGuard, Item, layout(), Msg, NotificationsPage, now_secs(), parse(), relative() (+20 more)

### Community 9 - "Lista attività"
Cohesion: 0.11
Nodes (17): Hit, Layout, Mode, Task, TodoPage, DrawCtx, Host, Mouse (+9 more)

### Community 10 - "Contratto delle pagine"
Cohesion: 0.07
Nodes (22): Fn, Gfx, Activity, build(), Host, Mouse, Page, Placeholder (+14 more)

### Community 11 - "Meteo"
Cohesion: 0.12
Nodes (29): as_f32(), entry_details(), entry_symbol(), entry_temp(), feels_like_c(), fetch_forecast(), fetch_location(), fetch_weather() (+21 more)

### Community 12 - "Utilizzo Claude"
Cohesion: 0.14
Nodes (24): ClaudePage, draw_window_card(), format_countdown(), format_weekly_reset(), json_i64(), local_offset_secs(), now_utc_ms(), parse_usage() (+16 more)

### Community 13 - "Impostazioni"
Cohesion: 0.15
Nodes (16): is_row_on(), Kind, Row, SettingsPage, BTreeMap, DrawCtx, Host, Mouse (+8 more)

### Community 14 - "Statistiche del sistema"
Cohesion: 0.16
Nodes (17): FILETIME, cpu_color(), draw_gauge(), filetime_u64(), RamStats, read_cpu_times(), read_ram(), SystemPage (+9 more)

### Community 15 - "Mascotte"
Cohesion: 0.21
Nodes (7): Mascot, Mood, DrawCtx, Point, Rect, Self, Spring

### Community 16 - "Worker e messaggi"
Cohesion: 0.16
Nodes (15): F, Mutex, drain_commands(), Peek, Color, HWND, Self, String (+7 more)

### Community 17 - "Persistenza locale"
Cohesion: 0.28
Nodes (15): data_dir(), dir_from_env(), local_dir(), read_kv(), read_lines(), read_text(), BTreeMap, Option (+7 more)

### Community 18 - "Client HTTP"
Cohesion: 0.40
Nodes (9): encode(), fetch_bytes(), get_bytes(), get_json(), get_string(), Result, String, Value (+1 more)

### Community 20 - "Avvio applicazione"
Cohesion: 0.40
Nodes (4): Error, main(), Box, Result

### Community 21 - "Profilo iniziale"
Cohesion: 0.60
Nodes (4): first_run_creates_profile_and_preserves_subsequent_user_choices(), initialize_at(), Path, Result

## Knowledge Gaps
- **130 isolated node(s):** `Self`, `IDropTarget`, `Kind`, `Kind`, `IDropTarget_Impl` (+125 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Gfx` connect `Contratto delle pagine` to `Risultati sportivi`, `Finestra e interazioni`, `Media e testi`, `Deposito e trascinamento`, `Calendario`, `Timer focus`, `Notifiche Windows`, `Lista attività`, `Meteo`, `Utilizzo Claude`, `Impostazioni`, `Statistiche del sistema`, `Mascotte`?**
  _High betweenness centrality (0.750) - this node is a cross-community bridge._
- **Why does `App` connect `Finestra e interazioni` to `Worker e messaggi`, `Contratto delle pagine`?**
  _High betweenness centrality (0.127) - this node is a cross-community bridge._
- **Why does `RefCell` connect `Grafica Direct2D` to `Finestra e interazioni`?**
  _High betweenness centrality (0.091) - this node is a cross-community bridge._
- **What connects `Self`, `IDropTarget`, `Kind` to the rest of the system?**
  _130 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Risultati sportivi` be split into smaller, more focused modules?**
  _Cohesion score 0.07149122807017544 - nodes in this community are weakly interconnected._
- **Should `Finestra e interazioni` be split into smaller, more focused modules?**
  _Cohesion score 0.05811965811965812 - nodes in this community are weakly interconnected._
- **Should `Grafica Direct2D` be split into smaller, more focused modules?**
  _Cohesion score 0.06127206127206127 - nodes in this community are weakly interconnected._
