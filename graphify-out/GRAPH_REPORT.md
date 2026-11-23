# Current code map

Local AST only. No application code, LLM backend or network command was run by this script. Source hashes are in SOURCE_SNAPSHOT.json. The map may miss dynamic dispatch, COM/FFI and runtime paths; it is not proof of correctness or safety. Check the hashes before treating it as current. Generated artifacts may be replaced on the next refresh; keep historical reviews under docs/review instead.

# Graph Report - src  (2026-11-23)

## Corpus Check
- Corpus is ~41,140 words - fits in a single context window. You may not need a graph.

## Summary
- 1019 nodes · 2388 edges · 25 communities (24 shown, 1 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 13 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_shell|shell]]
- [[_COMMUNITY_football|football]]
- [[_COMMUNITY_gfx|gfx]]
- [[_COMMUNITY_media|media]]
- [[_COMMUNITY_shelf|shelf]]
- [[_COMMUNITY_desktop_clock|desktop_clock]]
- [[_COMMUNITY_droptarget|droptarget]]
- [[_COMMUNITY_calendar|calendar]]
- [[_COMMUNITY_weather|weather]]
- [[_COMMUNITY_focus|focus]]
- [[_COMMUNITY_notifications|notifications]]
- [[_COMMUNITY_settings|settings]]
- [[_COMMUNITY_todo|todo]]
- [[_COMMUNITY_mascot|mascot]]
- [[_COMMUNITY_claude|claude]]
- [[_COMMUNITY_theme|theme]]
- [[_COMMUNITY_store|store]]
- [[_COMMUNITY_system|system]]
- [[_COMMUNITY_profile|profile]]
- [[_COMMUNITY_mod|mod]]
- [[_COMMUNITY_navigation|navigation]]
- [[_COMMUNITY_worker|worker]]
- [[_COMMUNITY_http|http]]
- [[_COMMUNITY_anim|anim]]
- [[_COMMUNITY_main|main]]

## God Nodes (most connected - your core abstractions)
1. `App` - 66 edges
2. `MediaPage` - 32 edges
3. `ShelfPage` - 31 edges
4. `CalendarPage` - 30 edges
5. `wndproc()` - 30 edges
6. `Event` - 27 edges
7. `TodoPage` - 27 edges
8. `DrawCtx` - 25 edges
9. `Rect` - 23 edges
10. `Option` - 22 edges

## Surprising Connections (you probably didn't know these)
- `with_app()` --calls--> `F`  [INFERRED]
  src/shell.rs → src/worker.rs
- `read_files()` --calls--> `HDROP`  [INFERRED]
  src/droptarget.rs → src/shell.rs
- `build()` --calls--> `enabled()`  [INFERRED]
  src/pages/mod.rs → src/desktop_clock.rs
- `contrast()` --calls--> `luminance()`  [INFERRED]
  src/theme.rs → src/pages/football.rs

## Import Cycles
- 1-file cycle: `src/desktop_clock.rs -> src/desktop_clock.rs`
- 1-file cycle: `src/droptarget.rs -> src/droptarget.rs`
- 1-file cycle: `src/gfx.rs -> src/gfx.rs`
- 1-file cycle: `src/mascot.rs -> src/mascot.rs`
- 1-file cycle: `src/shell.rs -> src/shell.rs`
- 1-file cycle: `src/pages/calendar.rs -> src/pages/calendar.rs`
- 1-file cycle: `src/pages/claude.rs -> src/pages/claude.rs`
- 1-file cycle: `src/pages/football.rs -> src/pages/football.rs`
- 1-file cycle: `src/pages/media.rs -> src/pages/media.rs`
- 1-file cycle: `src/pages/mod.rs -> src/pages/mod.rs`
- 1-file cycle: `src/pages/settings.rs -> src/pages/settings.rs`
- 1-file cycle: `src/pages/shelf.rs -> src/pages/shelf.rs`
- 1-file cycle: `src/pages/system.rs -> src/pages/system.rs`
- 1-file cycle: `src/pages/todo.rs -> src/pages/todo.rs`
- 1-file cycle: `src/pages/weather.rs -> src/pages/weather.rs`
- 1-file cycle: `src/profile.rs -> src/profile.rs`
- 1-file cycle: `src/store.rs -> src/store.rs`
- 1-file cycle: `src/worker.rs -> src/worker.rs`

## Communities (25 total, 1 thin omitted)

### Community 0 - "shell"
Cohesion: 0.05
Nodes (48): DropData, DropHandler, HDROP, HICON, Mascot, add_tray_icon(), App, app_icon() (+40 more)

### Community 1 - "football"
Cohesion: 0.07
Nodes (74): apply_summary(), badge_colors(), Change, changes(), crest_key(), Crests, days_to_ymd(), detects_goals_kickoff_and_full_time() (+66 more)

### Community 2 - "gfx"
Cohesion: 0.06
Nodes (33): D2D1_COLOR_F, D2D_RECT_F, ID2D1Bitmap, ID2D1DeviceContext, ID2D1Factory1, ID2D1GradientStopCollection, ID2D1SolidColorBrush, ID2D1StrokeStyle (+25 more)

### Community 3 - "media"
Cohesion: 0.07
Nodes (50): AtomicU64, Manager, MediaProps, Cmd, ease_out_cubic(), fetch_lyrics(), fmt_time(), Hit (+42 more)

### Community 4 - "shelf"
Cohesion: 0.07
Nodes (38): AtomicU32, BOOL, GUID, HRESULT, IDropSource, IDropSource_Vtbl, drag_out(), DropSource (+30 more)

### Community 5 - "desktop_clock"
Cohesion: 0.08
Nodes (45): COLORREF, From, HDC, HFONT, clock_text(), ClockWindow, color_ref(), desktop_class() (+37 more)

### Community 6 - "droptarget"
Cohesion: 0.09
Nodes (39): FORMATETC, IDataObject, IDropTarget, IDropTarget_Impl, Kind, POINTL, as_web_url(), clipboard_text() (+31 more)

### Community 7 - "calendar"
Cohesion: 0.10
Nodes (22): CalendarPage, Date, days_in_month(), Event, Hit, is_leap(), Layout, local_today() (+14 more)

### Community 8 - "weather"
Cohesion: 0.09
Nodes (38): FnMut, a_stop_before_start_prevents_the_first_refresh(), as_f32(), entry_details(), entry_symbol(), entry_temp(), feels_like_c(), fetch_forecast() (+30 more)

### Community 9 - "focus"
Cohesion: 0.11
Nodes (20): Btn, FocusPage, format_mm_ss(), Layout, LocalDate, Phase, State, Activity (+12 more)

### Community 10 - "notifications"
Cohesion: 0.11
Nodes (28): EventGuard, Item, layout(), Msg, NotificationsPage, now_secs(), parse(), relative() (+20 more)

### Community 11 - "settings"
Cohesion: 0.13
Nodes (26): a_failed_save_does_not_apply_the_selection_and_a_retry_can_succeed(), appearance_cycles_and_changes_no_other_preference(), changed_settings(), invalid_appearance_uses_original_before_advancing(), is_row_on(), Kind, page(), Row (+18 more)

### Community 12 - "todo"
Cohesion: 0.11
Nodes (17): Hit, Layout, Mode, Task, TodoPage, DrawCtx, Host, Mouse (+9 more)

### Community 13 - "mascot"
Cohesion: 0.10
Nodes (14): Gfx, Outcome, TextField, Mascot, Mood, DrawCtx, Point, Rect (+6 more)

### Community 14 - "claude"
Cohesion: 0.14
Nodes (24): ClaudePage, draw_window_card(), format_countdown(), format_weekly_reset(), json_i64(), local_offset_secs(), now_utc_ms(), parse_usage() (+16 more)

### Community 15 - "theme"
Cohesion: 0.13
Nodes (22): Palette, accent(), apply(), bg(), blue(), border(), card(), card_hover() (+14 more)

### Community 16 - "store"
Cohesion: 0.17
Nodes (23): data_dir(), dir_from_env(), Fixture, local_dir(), read_failures_are_not_a_successful_empty_configuration(), read_kv(), read_kv_at(), read_lines() (+15 more)

### Community 17 - "system"
Cohesion: 0.16
Nodes (17): FILETIME, cpu_color(), draw_gauge(), filetime_u64(), RamStats, read_cpu_times(), read_ram(), SystemPage (+9 more)

### Community 18 - "profile"
Cohesion: 0.22
Nodes (18): File, a_directory_at_the_profile_path_fails_closed(), a_profile_created_during_staging_is_never_overwritten(), an_interrupted_write_leaves_no_final_profile_and_next_start_can_retry(), first_run_creates_profile_and_preserves_subsequent_user_choices(), initialize_at(), initialize_with(), profile_exists() (+10 more)

### Community 19 - "mod"
Cohesion: 0.14
Nodes (15): Activity, build(), Host, Mouse, Page, Placeholder, Box, DrawCtx (+7 more)

### Community 20 - "navigation"
Cohesion: 0.24
Nodes (13): feature_glyphs_are_static_native_icons_without_a_phone_affordance(), glyph(), hit_testing_selects_every_tab_and_never_the_gaps_or_padding(), invalid_or_overflowing_coordinates_are_not_a_layout(), key_step(), Layout, panel(), Option (+5 more)

### Community 21 - "worker"
Cohesion: 0.18
Nodes (13): F, Mutex, drain_commands(), Peek, Color, HWND, Self, String (+5 more)

### Community 22 - "http"
Cohesion: 0.40
Nodes (9): encode(), fetch_bytes(), get_bytes(), get_json(), get_string(), Result, String, Value (+1 more)

### Community 24 - "main"
Cohesion: 0.40
Nodes (4): Error, main(), Box, Result

## Knowledge Gaps
- **151 isolated node(s):** `Self`, `From`, `HFONT`, `RefCell`, `Box` (+146 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What connects `Self`, `From`, `HFONT` to the rest of the system?**
  _151 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `shell` be split into smaller, more focused modules?**
  _Cohesion score 0.054254711593375214 - nodes in this community are weakly interconnected._
- **Should `football` be split into smaller, more focused modules?**
  _Cohesion score 0.07192982456140351 - nodes in this community are weakly interconnected._
- **Should `gfx` be split into smaller, more focused modules?**
  _Cohesion score 0.06260406260406261 - nodes in this community are weakly interconnected._
- **Should `media` be split into smaller, more focused modules?**
  _Cohesion score 0.06997408367271381 - nodes in this community are weakly interconnected._
- **Should `shelf` be split into smaller, more focused modules?**
  _Cohesion score 0.06554019457245264 - nodes in this community are weakly interconnected._
- **Should `desktop_clock` be split into smaller, more focused modules?**
  _Cohesion score 0.08051948051948052 - nodes in this community are weakly interconnected._