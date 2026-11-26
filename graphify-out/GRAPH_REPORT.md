# Current code map

Local AST only. No application code, LLM backend or network command was run by this script. Source hashes are in SOURCE_SNAPSHOT.json. The map may miss dynamic dispatch, COM/FFI and runtime paths; it is not proof of correctness or safety. Check the hashes before treating it as current. Generated artifacts may be replaced on the next refresh; keep historical reviews under docs/review instead.

# Graph Report - src  (2026-11-26)

## Corpus Check
- 45 files · ~71,539 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1945 nodes · 4815 edges · 42 communities (41 shown, 1 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 41 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_shell|shell]]
- [[_COMMUNITY_football|football]]
- [[_COMMUNITY_windows|windows]]
- [[_COMMUNITY_gfx|gfx]]
- [[_COMMUNITY_media|media]]
- [[_COMMUNITY_phone|phone]]
- [[_COMMUNITY_wire|wire]]
- [[_COMMUNITY_shelf|shelf]]
- [[_COMMUNITY_protocol|protocol]]
- [[_COMMUNITY_capture|capture]]
- [[_COMMUNITY_call_stt|call_stt]]
- [[_COMMUNITY_mod|mod]]
- [[_COMMUNITY_call_coach|call_coach]]
- [[_COMMUNITY_desktop_clock|desktop_clock]]
- [[_COMMUNITY_codex_bridge|codex_bridge]]
- [[_COMMUNITY_assist|assist]]
- [[_COMMUNITY_droptarget|droptarget]]
- [[_COMMUNITY_calendar|calendar]]
- [[_COMMUNITY_call_audio|call_audio]]
- [[_COMMUNITY_weather|weather]]
- [[_COMMUNITY_transport|transport]]
- [[_COMMUNITY_settings|settings]]
- [[_COMMUNITY_focus|focus]]
- [[_COMMUNITY_notifications|notifications]]
- [[_COMMUNITY_todo|todo]]
- [[_COMMUNITY_phone|phone]]
- [[_COMMUNITY_claude|claude]]
- [[_COMMUNITY_assist_metrics|assist_metrics]]
- [[_COMMUNITY_theme|theme]]
- [[_COMMUNITY_call_session|call_session]]
- [[_COMMUNITY_store|store]]
- [[_COMMUNITY_phone_qr|phone_qr]]
- [[_COMMUNITY_system|system]]
- [[_COMMUNITY_profile|profile]]
- [[_COMMUNITY_navigation|navigation]]
- [[_COMMUNITY_assist_layout|assist_layout]]
- [[_COMMUNITY_worker|worker]]
- [[_COMMUNITY_codex_diagnostic|codex_diagnostic]]
- [[_COMMUNITY_http|http]]
- [[_COMMUNITY_anim|anim]]
- [[_COMMUNITY_main|main]]

## God Nodes (most connected - your core abstractions)
1. `App` - 68 edges
2. `Assistant` - 32 edges
3. `MediaPage` - 32 edges
4. `ShelfPage` - 31 edges
5. `CalendarPage` - 30 edges
6. `wndproc()` - 30 edges
7. `State` - 29 edges
8. `OverlayManager` - 27 edges
9. `Event` - 27 edges
10. `TodoPage` - 27 edges

## Surprising Connections (you probably didn't know these)
- `wndproc()` --calls--> `HDC`  [INFERRED]
  src/assist_overlay.rs → src/desktop_clock.rs
- `with_app()` --calls--> `F`  [INFERRED]
  src/shell.rs → src/worker.rs
- `set_text()` --calls--> `PCWSTR`  [INFERRED]
  src/assist_overlay.rs → src/call_audio/windows.rs
- `create_controls()` --calls--> `PCWSTR`  [INFERRED]
  src/assist_overlay.rs → src/call_audio/windows.rs
- `selected_live()` --calls--> `same_identity()`  [INFERRED]
  src/call_audio/windows.rs → src/call_audio.rs

## Import Cycles
- 1-file cycle: `src/assist.rs -> src/assist.rs`
- 1-file cycle: `src/call_audio/windows.rs -> src/call_audio/windows.rs`
- 1-file cycle: `src/assist_metrics.rs -> src/assist_metrics.rs`
- 1-file cycle: `src/assist_overlay.rs -> src/assist_overlay.rs`
- 1-file cycle: `src/codex_bridge/transport.rs -> src/codex_bridge/transport.rs`
- 1-file cycle: `src/call_coach.rs -> src/call_coach.rs`
- 1-file cycle: `src/call_session.rs -> src/call_session.rs`
- 1-file cycle: `src/call_stt.rs -> src/call_stt.rs`
- 1-file cycle: `src/codex_bridge.rs -> src/codex_bridge.rs`
- 1-file cycle: `src/codex_diagnostic.rs -> src/codex_diagnostic.rs`
- 1-file cycle: `src/codex_server.rs -> src/codex_server.rs`
- 1-file cycle: `src/desktop_clock.rs -> src/desktop_clock.rs`
- 1-file cycle: `src/droptarget.rs -> src/droptarget.rs`
- 1-file cycle: `src/gfx.rs -> src/gfx.rs`
- 1-file cycle: `src/mascot.rs -> src/mascot.rs`
- 1-file cycle: `src/shell.rs -> src/shell.rs`
- 1-file cycle: `src/pages/calendar.rs -> src/pages/calendar.rs`
- 1-file cycle: `src/pages/claude.rs -> src/pages/claude.rs`
- 1-file cycle: `src/pages/football.rs -> src/pages/football.rs`
- 1-file cycle: `src/pages/media.rs -> src/pages/media.rs`

## Communities (42 total, 1 thin omitted)

### Community 0 - "assist_overlay"
Cohesion: 0.06
Nodes (83): Default, IntoIterator, Item, Monitor, MSG, OverlayWindow, PlacementError, Rc (+75 more)

### Community 1 - "shell"
Cohesion: 0.05
Nodes (49): Assistant, DropData, DropHandler, HDROP, HICON, Mascot, add_tray_icon(), App (+41 more)

### Community 2 - "football"
Cohesion: 0.07
Nodes (75): apply_summary(), badge_colors(), Change, changes(), crest_key(), Crests, days_to_ymd(), detects_goals_kickoff_and_full_time() (+67 more)

### Community 3 - "windows"
Cohesion: 0.06
Nodes (54): AudioSessionDisconnectReason, AudioSessionState, Com, copy_wide(), CreatedCallback, CreatedCallback_Impl, Device, DuckCallback (+46 more)

### Community 4 - "gfx"
Cohesion: 0.06
Nodes (33): D2D1_COLOR_F, D2D_RECT_F, ID2D1Bitmap, ID2D1DeviceContext, ID2D1Factory1, ID2D1GradientStopCollection, ID2D1SolidColorBrush, ID2D1StrokeStyle (+25 more)

### Community 5 - "media"
Cohesion: 0.07
Nodes (51): Manager, MediaProps, Cmd, ease_out_cubic(), fetch_lyrics(), fmt_time(), Hit, Layout (+43 more)

### Community 6 - "phone"
Cohesion: 0.08
Nodes (49): CertifiedKey, KeyPair, approval_requires_exact_live_pending_request(), directory_guard(), expire(), generate_identity(), hex(), Incoming (+41 more)

### Community 7 - "wire"
Cohesion: 0.10
Nodes (57): ClientConnection, body_json(), call(), call_with_cert(), cancelled(), client_stream(), copy_exact(), decode() (+49 more)

### Community 8 - "shelf"
Cohesion: 0.07
Nodes (38): AtomicU32, HRESULT, IDropSource, IDropSource_Vtbl, drag_out(), DropSource, ds_add_ref(), ds_give_feedback() (+30 more)

### Community 9 - "protocol"
Cohesion: 0.10
Nodes (43): BTreeSet, account_read_uses_no_token_refresh_and_blocks_api_key_actions(), answer_preserves_rpc_identity_and_is_not_a_new_turn(), create_error_releases_guard_and_account_change_invalidates_prompts(), create_is_explicit_single_flight_read_only_and_ack_registers_exact_id(), create_mismatch_cannot_be_reenabled_by_metadata_refresh_or_reload(), failed_or_closed_thread_releases_followup_guard_without_retry(), followup_uses_only_queue_and_never_steers() (+35 more)

### Community 10 - "capture"
Cohesion: 0.06
Nodes (45): Activation, activation_params(), ActivationSignal, Capture, check_windows(), Completion, Completion_Impl, decode_pcm() (+37 more)

### Community 11 - "call_stt"
Cohesion: 0.10
Nodes (41): cancellation_rejects_old_results_without_rewriting_capture_generation(), check_file(), Configuration, create_private_directory(), diagnostic_wav_rejects_truncation_and_wrong_format_before_inference(), DiagnosticResult, ensure_local_tree(), is_reparse() (+33 more)

### Community 12 - "mod"
Cohesion: 0.06
Nodes (32): Gfx, Activity, build(), Host, Mouse, Page, phone_enabled(), phone_needs_explicit_opt_in_even_for_legacy_profiles() (+24 more)

### Community 13 - "call_coach"
Cohesion: 0.12
Nodes (43): SocketAddr, Coach, Endpoint, endpoint_is_loopback_only_and_secrets_are_not_response_data(), escaped_transcript_context_and_path_inputs_stay_bounded(), handle(), is_reparse(), local_ipc_round_trip_is_bounded_and_stop_revokes_pointer() (+35 more)

### Community 14 - "desktop_clock"
Cohesion: 0.08
Nodes (46): From, HBRUSH, HDC, clock_text(), ClockWindow, color_ref(), desktop_class(), desktop_is_foreground() (+38 more)

### Community 15 - "codex_bridge"
Cohesion: 0.11
Nodes (39): Into, AccountState, AnswerKey, Approval, Bridge, check_local(), cli_executable(), Command (+31 more)

### Community 16 - "assist"
Cohesion: 0.08
Nodes (37): AudioMonitor, AudioSnapshot, CallSession, CallView, DashboardView, MetricsWorker, OverlayAction, OverlayAvailability (+29 more)

### Community 17 - "droptarget"
Cohesion: 0.09
Nodes (39): FORMATETC, IDataObject, IDropTarget, IDropTarget_Impl, Kind, POINTL, as_web_url(), clipboard_text() (+31 more)

### Community 18 - "calendar"
Cohesion: 0.10
Nodes (22): CalendarPage, Date, days_in_month(), Event, Hit, is_leap(), Layout, local_today() (+14 more)

### Community 19 - "call_audio"
Cohesion: 0.10
Nodes (32): HashSet, AudioChunk, AudioMonitor, AudioSnapshot, AudioTarget, CapturePhase, CaptureStatus, concurrent_stop_waits_for_confirm_state_before_revoking_generation() (+24 more)

### Community 20 - "weather"
Cohesion: 0.09
Nodes (38): a_stop_before_start_prevents_the_first_refresh(), as_f32(), entry_details(), entry_symbol(), entry_temp(), feels_like_c(), fetch_forecast(), fetch_location() (+30 more)

### Community 21 - "transport"
Cohesion: 0.10
Nodes (34): AtomicUsize, ChildStdout, BlockOnce, canonical_local(), discover_cli(), discover_socket(), kill(), Progress (+26 more)

### Community 22 - "settings"
Cohesion: 0.12
Nodes (29): a_failed_save_does_not_apply_the_selection_and_a_retry_can_succeed(), appearance_cycles_and_changes_no_other_preference(), assistant_monitor_cycles_only_the_supplied_secondary_devices(), changed_settings(), invalid_appearance_uses_original_before_advancing(), is_row_on(), Kind, page() (+21 more)

### Community 23 - "focus"
Cohesion: 0.11
Nodes (20): Btn, FocusPage, format_mm_ss(), Layout, LocalDate, Phase, State, Activity (+12 more)

### Community 24 - "notifications"
Cohesion: 0.11
Nodes (28): EventGuard, Item, layout(), Msg, NotificationsPage, now_secs(), parse(), relative() (+20 more)

### Community 25 - "todo"
Cohesion: 0.11
Nodes (17): Hit, Layout, Mode, Task, TodoPage, DrawCtx, Host, Mouse (+9 more)

### Community 26 - "phone"
Cohesion: 0.11
Nodes (20): buttons(), controls_fit_the_compact_body(), phone_page_is_inert_until_explicit_start(), PhonePage, PhoneSnapshot, QrWindow, DrawCtx, Host (+12 more)

### Community 27 - "claude"
Cohesion: 0.14
Nodes (24): ClaudePage, draw_window_card(), format_countdown(), format_weekly_reset(), json_i64(), local_offset_secs(), now_utc_ms(), parse_usage() (+16 more)

### Community 28 - "assist_metrics"
Cohesion: 0.12
Nodes (18): filetime(), Handle, MetricsWorker, normalized_cpu(), ResourceSnapshot, BTreeMap, Drop, Duration (+10 more)

### Community 29 - "theme"
Cohesion: 0.13
Nodes (21): Palette, accent(), apply(), bg(), blue(), border(), card(), card_hover() (+13 more)

### Community 30 - "call_session"
Cohesion: 0.14
Nodes (20): AudioChunk, Coach, Configuration, CallSession, continuous_speech_is_bounded_and_overlaps_at_cuts(), merge(), overlap_merging_keeps_new_speech_and_bounds_utf8(), BTreeMap (+12 more)

### Community 31 - "store"
Cohesion: 0.17
Nodes (23): data_dir(), dir_from_env(), Fixture, local_dir(), read_failures_are_not_a_successful_empty_configuration(), read_kv(), read_kv_at(), read_lines() (+15 more)

### Community 32 - "phone_qr"
Cohesion: 0.12
Nodes (20): QrCode, paint(), qr_geometry(), qr_preserves_integer_modules_and_four_module_border(), QrWindow, rgb(), Box, Cell (+12 more)

### Community 33 - "system"
Cohesion: 0.16
Nodes (17): FILETIME, cpu_color(), draw_gauge(), filetime_u64(), RamStats, read_cpu_times(), read_ram(), SystemPage (+9 more)

### Community 34 - "profile"
Cohesion: 0.22
Nodes (18): a_directory_at_the_profile_path_fails_closed(), a_profile_created_during_staging_is_never_overwritten(), an_interrupted_write_leaves_no_final_profile_and_next_start_can_retry(), first_run_creates_profile_and_preserves_subsequent_user_choices(), initialize_at(), initialize_with(), profile_exists(), publish_without_replacing() (+10 more)

### Community 35 - "navigation"
Cohesion: 0.24
Nodes (13): feature_glyphs_are_static_native_icons(), glyph(), hit_testing_selects_every_tab_and_never_the_gaps_or_padding(), invalid_or_overflowing_coordinates_are_not_a_layout(), key_step(), Layout, panel(), Option (+5 more)

### Community 36 - "assist_layout"
Cohesion: 0.22
Nodes (11): corners_fit_negative_and_above_primary_coordinates_without_overlap(), invalid_dpi_small_workspaces_and_extreme_arithmetic_fail_closed(), Monitor, overlay_rects(), OverlayRects, PlacementError, Option, Result (+3 more)

### Community 37 - "worker"
Cohesion: 0.20
Nodes (12): F, drain_commands(), Peek, Color, HWND, Self, String, Vec (+4 more)

### Community 38 - "codex_diagnostic"
Cohesion: 0.33
Nodes (10): connect(), Bridge, Fn, Option, PathBuf, Result, Snapshot, String (+2 more)

### Community 39 - "http"
Cohesion: 0.40
Nodes (9): encode(), fetch_bytes(), get_bytes(), get_json(), get_string(), Result, String, Value (+1 more)

### Community 41 - "main"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

## Knowledge Gaps
- **309 isolated node(s):** `Self`, `OverlayManager`, `OverlayConfig`, `OverlayAvailability`, `StartWorker` (+304 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What connects `Self`, `OverlayManager`, `OverlayConfig` to the rest of the system?**
  _309 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `assist_overlay` be split into smaller, more focused modules?**
  _Cohesion score 0.057208237986270026 - nodes in this community are weakly interconnected._
- **Should `shell` be split into smaller, more focused modules?**
  _Cohesion score 0.053296703296703295 - nodes in this community are weakly interconnected._
- **Should `football` be split into smaller, more focused modules?**
  _Cohesion score 0.07066151202749141 - nodes in this community are weakly interconnected._
- **Should `windows` be split into smaller, more focused modules?**
  _Cohesion score 0.05612694681163679 - nodes in this community are weakly interconnected._
- **Should `gfx` be split into smaller, more focused modules?**
  _Cohesion score 0.06260406260406261 - nodes in this community are weakly interconnected._
- **Should `media` be split into smaller, more focused modules?**
  _Cohesion score 0.06846846846846846 - nodes in this community are weakly interconnected._