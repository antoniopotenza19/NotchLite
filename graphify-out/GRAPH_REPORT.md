# Current code map

Local AST only. No application code, LLM backend or network command was run by this script. Source hashes are in SOURCE_SNAPSHOT.json. The map may miss dynamic dispatch, COM/FFI and runtime paths; it is not proof of correctness or safety. Check the hashes before treating it as current. Generated artifacts may be replaced on the next refresh; keep historical reviews under docs/review instead.

# Graph Report - src  (2026-11-23)

## Corpus Check
- Corpus is ~48,425 words - fits in a single context window. You may not need a graph.

## Summary
- 1224 nodes · 2927 edges · 29 communities (28 shown, 1 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 21 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_shell|shell]]
- [[_COMMUNITY_football|football]]
- [[_COMMUNITY_gfx|gfx]]
- [[_COMMUNITY_media|media]]
- [[_COMMUNITY_phone|phone]]
- [[_COMMUNITY_wire|wire]]
- [[_COMMUNITY_shelf|shelf]]
- [[_COMMUNITY_desktop_clock|desktop_clock]]
- [[_COMMUNITY_droptarget|droptarget]]
- [[_COMMUNITY_calendar|calendar]]
- [[_COMMUNITY_weather|weather]]
- [[_COMMUNITY_focus|focus]]
- [[_COMMUNITY_settings|settings]]
- [[_COMMUNITY_notifications|notifications]]
- [[_COMMUNITY_todo|todo]]
- [[_COMMUNITY_mascot|mascot]]
- [[_COMMUNITY_phone|phone]]
- [[_COMMUNITY_claude|claude]]
- [[_COMMUNITY_theme|theme]]
- [[_COMMUNITY_store|store]]
- [[_COMMUNITY_phone_qr|phone_qr]]
- [[_COMMUNITY_system|system]]
- [[_COMMUNITY_mod|mod]]
- [[_COMMUNITY_profile|profile]]
- [[_COMMUNITY_navigation|navigation]]
- [[_COMMUNITY_worker|worker]]
- [[_COMMUNITY_http|http]]
- [[_COMMUNITY_anim|anim]]
- [[_COMMUNITY_main|main]]

## God Nodes (most connected - your core abstractions)
1. `App` - 67 edges
2. `MediaPage` - 32 edges
3. `ShelfPage` - 31 edges
4. `CalendarPage` - 30 edges
5. `wndproc()` - 30 edges
6. `Event` - 27 edges
7. `TodoPage` - 27 edges
8. `DrawCtx` - 25 edges
9. `PhonePage` - 24 edges
10. `Rect` - 23 edges

## Surprising Connections (you probably didn't know these)
- `with_app()` --calls--> `F`  [INFERRED]
  src/shell.rs → src/worker.rs
- `read_files()` --calls--> `HDROP`  [INFERRED]
  src/droptarget.rs → src/shell.rs
- `build()` --calls--> `enabled()`  [INFERRED]
  src/pages/mod.rs → src/desktop_clock.rs
- `pair()` --calls--> `expire()`  [INFERRED]
  src/phone/wire.rs → src/phone.rs
- `pair()` --calls--> `random_hex()`  [INFERRED]
  src/phone/wire.rs → src/phone.rs

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
- 1-file cycle: `src/pages/phone.rs -> src/pages/phone.rs`
- 1-file cycle: `src/pages/settings.rs -> src/pages/settings.rs`
- 1-file cycle: `src/pages/shelf.rs -> src/pages/shelf.rs`
- 1-file cycle: `src/pages/system.rs -> src/pages/system.rs`
- 1-file cycle: `src/pages/todo.rs -> src/pages/todo.rs`
- 1-file cycle: `src/pages/weather.rs -> src/pages/weather.rs`
- 1-file cycle: `src/phone/wire.rs -> src/phone/wire.rs`
- 1-file cycle: `src/phone.rs -> src/phone.rs`
- 1-file cycle: `src/phone_qr.rs -> src/phone_qr.rs`
- 1-file cycle: `src/profile.rs -> src/profile.rs`

## Communities (29 total, 1 thin omitted)

### Community 0 - "shell"
Cohesion: 0.05
Nodes (47): DropData, DropHandler, HDROP, HICON, Mascot, add_tray_icon(), App, app_icon() (+39 more)

### Community 1 - "football"
Cohesion: 0.07
Nodes (74): apply_summary(), badge_colors(), Change, changes(), crest_key(), Crests, days_to_ymd(), detects_goals_kickoff_and_full_time() (+66 more)

### Community 2 - "gfx"
Cohesion: 0.06
Nodes (33): D2D1_COLOR_F, D2D_RECT_F, ID2D1Bitmap, ID2D1DeviceContext, ID2D1Factory1, ID2D1GradientStopCollection, ID2D1SolidColorBrush, ID2D1StrokeStyle (+25 more)

### Community 3 - "media"
Cohesion: 0.07
Nodes (51): AtomicU64, Manager, MediaProps, Cmd, ease_out_cubic(), fetch_lyrics(), fmt_time(), Hit (+43 more)

### Community 4 - "phone"
Cohesion: 0.08
Nodes (50): CertifiedKey, JoinHandle, KeyPair, MutexGuard, approval_requires_exact_live_pending_request(), directory_guard(), expire(), generate_identity() (+42 more)

### Community 5 - "wire"
Cohesion: 0.10
Nodes (56): ClientConnection, body_json(), call(), call_with_cert(), cancelled(), client_stream(), copy_exact(), decode() (+48 more)

### Community 6 - "shelf"
Cohesion: 0.07
Nodes (38): AtomicU32, BOOL, GUID, HRESULT, IDropSource, IDropSource_Vtbl, drag_out(), DropSource (+30 more)

### Community 7 - "desktop_clock"
Cohesion: 0.08
Nodes (45): From, HDC, clock_text(), ClockWindow, color_ref(), desktop_class(), desktop_is_foreground(), drain_pending_shutdown() (+37 more)

### Community 8 - "droptarget"
Cohesion: 0.09
Nodes (39): FORMATETC, IDataObject, IDropTarget, IDropTarget_Impl, Kind, POINTL, as_web_url(), clipboard_text() (+31 more)

### Community 9 - "calendar"
Cohesion: 0.10
Nodes (22): CalendarPage, Date, days_in_month(), Event, Hit, is_leap(), Layout, local_today() (+14 more)

### Community 10 - "weather"
Cohesion: 0.09
Nodes (38): a_stop_before_start_prevents_the_first_refresh(), as_f32(), entry_details(), entry_symbol(), entry_temp(), feels_like_c(), fetch_forecast(), fetch_location() (+30 more)

### Community 11 - "focus"
Cohesion: 0.11
Nodes (20): Btn, FocusPage, format_mm_ss(), Layout, LocalDate, Phase, State, Activity (+12 more)

### Community 12 - "settings"
Cohesion: 0.12
Nodes (27): a_failed_save_does_not_apply_the_selection_and_a_retry_can_succeed(), appearance_cycles_and_changes_no_other_preference(), changed_settings(), invalid_appearance_uses_original_before_advancing(), is_row_on(), Kind, page(), phone_is_opt_in_and_other_settings_preserve_that_choice() (+19 more)

### Community 13 - "notifications"
Cohesion: 0.11
Nodes (28): EventGuard, Item, layout(), Msg, NotificationsPage, now_secs(), parse(), relative() (+20 more)

### Community 14 - "todo"
Cohesion: 0.11
Nodes (17): Hit, Layout, Mode, Task, TodoPage, DrawCtx, Host, Mouse (+9 more)

### Community 15 - "mascot"
Cohesion: 0.10
Nodes (14): Gfx, Outcome, TextField, Mascot, Mood, DrawCtx, Point, Rect (+6 more)

### Community 16 - "phone"
Cohesion: 0.11
Nodes (20): buttons(), controls_fit_the_compact_body(), phone_page_is_inert_until_explicit_start(), PhonePage, PhoneSnapshot, QrWindow, DrawCtx, Host (+12 more)

### Community 17 - "claude"
Cohesion: 0.14
Nodes (24): ClaudePage, draw_window_card(), format_countdown(), format_weekly_reset(), json_i64(), local_offset_secs(), now_utc_ms(), parse_usage() (+16 more)

### Community 18 - "theme"
Cohesion: 0.13
Nodes (22): Palette, accent(), apply(), bg(), blue(), border(), card(), card_hover() (+14 more)

### Community 19 - "store"
Cohesion: 0.17
Nodes (23): data_dir(), dir_from_env(), Fixture, local_dir(), read_failures_are_not_a_successful_empty_configuration(), read_kv(), read_kv_at(), read_lines() (+15 more)

### Community 20 - "phone_qr"
Cohesion: 0.12
Nodes (20): QrCode, paint(), qr_geometry(), qr_preserves_integer_modules_and_four_module_border(), QrWindow, rgb(), Box, Cell (+12 more)

### Community 21 - "system"
Cohesion: 0.16
Nodes (17): FILETIME, cpu_color(), draw_gauge(), filetime_u64(), RamStats, read_cpu_times(), read_ram(), SystemPage (+9 more)

### Community 22 - "mod"
Cohesion: 0.13
Nodes (18): Activity, build(), Host, Mouse, Page, phone_enabled(), phone_needs_explicit_opt_in_even_for_legacy_profiles(), Placeholder (+10 more)

### Community 23 - "profile"
Cohesion: 0.22
Nodes (18): a_directory_at_the_profile_path_fails_closed(), a_profile_created_during_staging_is_never_overwritten(), an_interrupted_write_leaves_no_final_profile_and_next_start_can_retry(), first_run_creates_profile_and_preserves_subsequent_user_choices(), initialize_at(), initialize_with(), profile_exists(), publish_without_replacing() (+10 more)

### Community 24 - "navigation"
Cohesion: 0.24
Nodes (13): feature_glyphs_are_static_native_icons(), glyph(), hit_testing_selects_every_tab_and_never_the_gaps_or_padding(), invalid_or_overflowing_coordinates_are_not_a_layout(), key_step(), Layout, panel(), Option (+5 more)

### Community 25 - "worker"
Cohesion: 0.18
Nodes (13): F, drain_commands(), Peek, Color, HWND, Self, String, Vec (+5 more)

### Community 26 - "http"
Cohesion: 0.40
Nodes (9): encode(), fetch_bytes(), get_bytes(), get_json(), get_string(), Result, String, Value (+1 more)

### Community 28 - "main"
Cohesion: 0.40
Nodes (4): Error, main(), Box, Result

## Knowledge Gaps
- **188 isolated node(s):** `Self`, `From`, `HFONT`, `RefCell`, `Box` (+183 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `with_app()` connect `shell` to `worker`?**
  _High betweenness centrality (0.001) - this node is a cross-community bridge._
- **Why does `App` connect `shell` to `worker`, `mascot`?**
  _High betweenness centrality (0.001) - this node is a cross-community bridge._
- **What connects `Self`, `From`, `HFONT` to the rest of the system?**
  _188 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `shell` be split into smaller, more focused modules?**
  _Cohesion score 0.05463544641157434 - nodes in this community are weakly interconnected._
- **Should `football` be split into smaller, more focused modules?**
  _Cohesion score 0.07192982456140351 - nodes in this community are weakly interconnected._
- **Should `gfx` be split into smaller, more focused modules?**
  _Cohesion score 0.06260406260406261 - nodes in this community are weakly interconnected._
- **Should `media` be split into smaller, more focused modules?**
  _Cohesion score 0.06846846846846846 - nodes in this community are weakly interconnected._