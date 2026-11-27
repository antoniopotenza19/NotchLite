# Current code map

Local AST only. No application code, LLM backend or network command was run by this script. Source hashes are in SOURCE_SNAPSHOT.json. The map may miss dynamic dispatch, COM/FFI and runtime paths; it is not proof of correctness or safety. Check the hashes before treating it as current. Generated artifacts may be replaced on the next refresh; keep historical reviews under docs/review instead.

# Graph Report - src  (2026-11-27)

## Corpus Check
- 50 files · ~79,786 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2198 nodes · 5524 edges · 56 communities (55 shown, 1 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 47 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_shell|shell]]
- [[_COMMUNITY_windows|windows]]
- [[_COMMUNITY_football|football]]
- [[_COMMUNITY_gfx|gfx]]
- [[_COMMUNITY_media|media]]
- [[_COMMUNITY_call_stt|call_stt]]
- [[_COMMUNITY_phone|phone]]
- [[_COMMUNITY_wire|wire]]
- [[_COMMUNITY_call_coach|call_coach]]
- [[_COMMUNITY_shelf|shelf]]
- [[_COMMUNITY_protocol|protocol]]
- [[_COMMUNITY_transport|transport]]
- [[_COMMUNITY_assist|assist]]
- [[_COMMUNITY_desktop_clock|desktop_clock]]
- [[_COMMUNITY_call_audio|call_audio]]
- [[_COMMUNITY_codex_bridge|codex_bridge]]
- [[_COMMUNITY_droptarget|droptarget]]
- [[_COMMUNITY_calendar|calendar]]
- [[_COMMUNITY_call_ai|call_ai]]
- [[_COMMUNITY_capture|capture]]
- [[_COMMUNITY_weather|weather]]
- [[_COMMUNITY_settings|settings]]
- [[_COMMUNITY_focus|focus]]
- [[_COMMUNITY_todo|todo]]
- [[_COMMUNITY_notifications|notifications]]
- [[_COMMUNITY_call_session|call_session]]
- [[_COMMUNITY_phone|phone]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_theme|theme]]
- [[_COMMUNITY_assist_layout|assist_layout]]
- [[_COMMUNITY_claude|claude]]
- [[_COMMUNITY_window_policy|window_policy]]
- [[_COMMUNITY_assist_metrics|assist_metrics]]
- [[_COMMUNITY_store|store]]
- [[_COMMUNITY_phone_qr|phone_qr]]
- [[_COMMUNITY_system|system]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_mod|mod]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_mascot|mascot]]
- [[_COMMUNITY_profile|profile]]
- [[_COMMUNITY_codex_server|codex_server]]
- [[_COMMUNITY_notch_monitor|notch_monitor]]
- [[_COMMUNITY_navigation|navigation]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_textfield|textfield]]
- [[_COMMUNITY_worker|worker]]
- [[_COMMUNITY_call_panel_size|call_panel_size]]
- [[_COMMUNITY_assist_glass|assist_glass]]
- [[_COMMUNITY_codex_diagnostic|codex_diagnostic]]
- [[_COMMUNITY_http|http]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_anim|anim]]
- [[_COMMUNITY_assist_overlay|assist_overlay]]
- [[_COMMUNITY_main|main]]

## God Nodes (most connected - your core abstractions)
1. `App` - 69 edges
2. `Assistant` - 33 edges
3. `MediaPage` - 32 edges
4. `ShelfPage` - 31 edges
5. `wndproc()` - 31 edges
6. `OverlayManager` - 30 edges
7. `CalendarPage` - 30 edges
8. `State` - 29 edges
9. `Event` - 27 edges
10. `TodoPage` - 27 edges

## Surprising Connections (you probably didn't know these)
- `create_controls()` --calls--> `WINDOW_EX_STYLE`  [INFERRED]
  src/assist_overlay.rs → src/window_policy.rs
- `with_app()` --calls--> `F`  [INFERRED]
  src/shell.rs → src/worker.rs
- `set_text()` --calls--> `PCWSTR`  [INFERRED]
  src/assist_overlay.rs → src/call_audio/windows.rs
- `create_controls()` --calls--> `PCWSTR`  [INFERRED]
  src/assist_overlay.rs → src/call_audio/windows.rs
- `layout_controls()` --calls--> `place()`  [INFERRED]
  src/assist_overlay.rs → src/notch_monitor.rs

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
- 1-file cycle: `src/notch_monitor.rs -> src/notch_monitor.rs`
- 1-file cycle: `src/pages/calendar.rs -> src/pages/calendar.rs`
- 1-file cycle: `src/pages/claude.rs -> src/pages/claude.rs`
- 1-file cycle: `src/pages/football.rs -> src/pages/football.rs`

## Communities (56 total, 1 thin omitted)

### Community 0 - "shell"
Cohesion: 0.05
Nodes (50): Assistant, DropData, DropHandler, Gfx, HDROP, HICON, Mascot, add_tray_icon() (+42 more)

### Community 1 - "windows"
Cohesion: 0.05
Nodes (66): AudioSessionDisconnectReason, AudioSessionState, active_teams_can_offer_consent_without_a_new_duck_event_but_cannot_capture(), Com, copy_wide(), CreatedCallback, CreatedCallback_Impl, Device (+58 more)

### Community 2 - "football"
Cohesion: 0.07
Nodes (74): apply_summary(), badge_colors(), Change, changes(), crest_key(), Crests, days_to_ymd(), detects_goals_kickoff_and_full_time() (+66 more)

### Community 3 - "gfx"
Cohesion: 0.06
Nodes (35): D2D1_COLOR_F, D2D_RECT_F, ID2D1Bitmap, ID2D1DeviceContext, ID2D1Factory1, ID2D1GradientStopCollection, ID2D1PathGeometry, ID2D1SolidColorBrush (+27 more)

### Community 4 - "media"
Cohesion: 0.07
Nodes (51): Manager, MediaProps, Cmd, ease_out_cubic(), fetch_lyrics(), fmt_time(), Hit, Layout (+43 more)

### Community 5 - "call_stt"
Cohesion: 0.08
Nodes (50): ChildStderr, asr_environment_override(), Backend, cancellation_rejects_old_results_without_rewriting_capture_generation(), check_file(), Configuration, create_private_directory(), diagnostic_wav_rejects_truncation_and_wrong_format_before_inference() (+42 more)

### Community 6 - "phone"
Cohesion: 0.08
Nodes (48): CertifiedKey, KeyPair, approval_requires_exact_live_pending_request(), directory_guard(), expire(), generate_identity(), hex(), Incoming (+40 more)

### Community 7 - "wire"
Cohesion: 0.10
Nodes (58): ClientConnection, body_json(), call(), call_with_cert(), cancelled(), client_stream(), copy_exact(), decode() (+50 more)

### Community 8 - "call_coach"
Cohesion: 0.11
Nodes (45): SocketAddr, Coach, Endpoint, endpoint_is_loopback_only_and_secrets_are_not_response_data(), escaped_transcript_context_and_path_inputs_stay_bounded(), handle(), is_reparse(), local_ipc_round_trip_is_bounded_and_stop_revokes_pointer() (+37 more)

### Community 9 - "shelf"
Cohesion: 0.07
Nodes (38): AtomicU32, HRESULT, IDropSource, IDropSource_Vtbl, drag_out(), DropSource, ds_add_ref(), ds_give_feedback() (+30 more)

### Community 10 - "protocol"
Cohesion: 0.10
Nodes (44): BTreeSet, account_read_uses_no_token_refresh_and_blocks_api_key_actions(), answer_preserves_rpc_identity_and_is_not_a_new_turn(), create_error_releases_guard_and_account_change_invalidates_prompts(), create_is_explicit_single_flight_read_only_and_ack_registers_exact_id(), create_mismatch_cannot_be_reenabled_by_metadata_refresh_or_reload(), failed_or_closed_thread_releases_followup_guard_without_retry(), followup_uses_only_queue_and_never_steers() (+36 more)

### Community 11 - "transport"
Cohesion: 0.09
Nodes (40): ChildStdout, BlockOnce, canonical_local(), discover_cli(), discover_socket(), kill(), Progress, ProxyIo (+32 more)

### Community 12 - "assist"
Cohesion: 0.08
Nodes (39): AudioMonitor, AudioSnapshot, CallSession, CallView, DashboardView, MetricsWorker, OverlayAction, OverlayAvailability (+31 more)

### Community 13 - "desktop_clock"
Cohesion: 0.08
Nodes (45): From, clock_text(), ClockWindow, color_ref(), desktop_class(), desktop_is_foreground(), drain_pending_shutdown(), enabled() (+37 more)

### Community 14 - "call_audio"
Cohesion: 0.09
Nodes (35): AudioChunk, AudioMonitor, AudioSnapshot, AudioTarget, CapturePhase, CaptureStatus, concurrent_stop_waits_for_confirm_state_before_revoking_generation(), declined_candidates_stay_declined_only_for_that_identity() (+27 more)

### Community 15 - "codex_bridge"
Cohesion: 0.11
Nodes (38): Into, AccountState, AnswerKey, Approval, Bridge, check_local(), cli_executable(), Command (+30 more)

### Community 16 - "droptarget"
Cohesion: 0.09
Nodes (39): FORMATETC, IDataObject, IDropTarget, IDropTarget_Impl, Kind, POINTL, as_web_url(), clipboard_text() (+31 more)

### Community 17 - "calendar"
Cohesion: 0.10
Nodes (22): CalendarPage, Date, days_in_month(), Event, Hit, is_leap(), Layout, local_today() (+14 more)

### Community 18 - "call_ai"
Cohesion: 0.11
Nodes (37): RpcTransport, add_tokens(), AiSnapshot, apply_sample(), bounded_reply(), call_prompt_is_bounded_utf8_even_with_json_escape_expansion(), drain_static_id(), Input (+29 more)

### Community 19 - "capture"
Cohesion: 0.08
Nodes (36): AtomicUsize, Activation, activation_params(), ActivationSignal, Capture, check_windows(), Completion, Completion_Impl (+28 more)

### Community 20 - "weather"
Cohesion: 0.09
Nodes (38): a_stop_before_start_prevents_the_first_refresh(), as_f32(), entry_details(), entry_symbol(), entry_temp(), feels_like_c(), fetch_forecast(), fetch_location() (+30 more)

### Community 21 - "settings"
Cohesion: 0.11
Nodes (32): a_failed_save_does_not_apply_the_selection_and_a_retry_can_succeed(), appearance_cycles_and_changes_no_other_preference(), assistant_monitor_cycles_only_the_supplied_secondary_devices(), changed_privacy_setting(), changed_settings(), invalid_appearance_uses_original_before_advancing(), is_row_on(), Kind (+24 more)

### Community 22 - "focus"
Cohesion: 0.11
Nodes (21): Btn, FocusPage, format_mm_ss(), Layout, LocalDate, Phase, State, Activity (+13 more)

### Community 23 - "todo"
Cohesion: 0.11
Nodes (17): Hit, Layout, Mode, Task, TodoPage, DrawCtx, Host, Mouse (+9 more)

### Community 24 - "notifications"
Cohesion: 0.11
Nodes (28): EventGuard, Item, layout(), Msg, NotificationsPage, now_secs(), parse(), relative() (+20 more)

### Community 25 - "call_session"
Cohesion: 0.11
Nodes (24): AudioChunk, Coach, Configuration, a_pause_after_a_forced_cut_does_not_transcribe_silence_or_overlap_alone(), above_noise_floor(), CallSession, continuous_speech_is_bounded_and_overlaps_at_cuts(), input_progress_distinguishes_quiet_pcm_and_never_counts_retained_overlap_twice() (+16 more)

### Community 26 - "phone"
Cohesion: 0.11
Nodes (20): buttons(), controls_fit_the_compact_body(), phone_page_is_inert_until_explicit_start(), PhonePage, PhoneSnapshot, QrWindow, DrawCtx, Host (+12 more)

### Community 27 - "assist_overlay"
Cohesion: 0.14
Nodes (29): DRAWITEMSTRUCT, IntoIterator, Item, close_action(), combo_items(), command(), create_controls(), draw_call_button() (+21 more)

### Community 28 - "assist_overlay"
Cohesion: 0.13
Nodes (24): MSG, an_invisible_interval_rearms_a_later_call_with_the_same_identity(), an_unregistered_dashboard_peer_is_cleared_without_querying_native_windows(), apply_call(), attention_color(), attention_tone(), AttentionTone, call_controls_after_update() (+16 more)

### Community 29 - "theme"
Cohesion: 0.13
Nodes (24): Palette, accent(), apply(), bg(), blue(), border(), card(), card_hover() (+16 more)

### Community 30 - "assist_layout"
Cohesion: 0.14
Nodes (24): another_monitor_or_an_edge_touch_does_not_consume_the_top_band(), call_controls_keep_diagnostics_context_and_consent_separate_at_minimum_and_portrait_sizes(), call_controls_rects(), call_reader_gives_live_text_the_space_instead_of_the_hidden_setup_form(), call_reader_rects(), CallControlsRects, CallReaderRects, corners_fit_negative_and_above_primary_coordinates_without_overlap() (+16 more)

### Community 31 - "claude"
Cohesion: 0.14
Nodes (24): ClaudePage, draw_window_card(), format_countdown(), format_weekly_reset(), json_i64(), local_offset_secs(), now_utc_ms(), parse_usage() (+16 more)

### Community 32 - "window_policy"
Cohesion: 0.12
Nodes (23): apply(), apply_inner(), Attempt, classify(), current(), metadata(), panel_style(), Policy (+15 more)

### Community 33 - "assist_metrics"
Cohesion: 0.12
Nodes (18): filetime(), Handle, MetricsWorker, normalized_cpu(), ResourceSnapshot, BTreeMap, Drop, Duration (+10 more)

### Community 34 - "store"
Cohesion: 0.17
Nodes (23): data_dir(), dir_from_env(), Fixture, local_dir(), read_failures_are_not_a_successful_empty_configuration(), read_kv(), read_kv_at(), read_lines() (+15 more)

### Community 35 - "phone_qr"
Cohesion: 0.12
Nodes (20): QrCode, paint(), qr_geometry(), qr_preserves_integer_modules_and_four_module_border(), QrWindow, rgb(), Box, Cell (+12 more)

### Community 36 - "system"
Cohesion: 0.16
Nodes (17): FILETIME, cpu_color(), draw_gauge(), filetime_u64(), RamStats, read_cpu_times(), read_ram(), SystemPage (+9 more)

### Community 37 - "assist_overlay"
Cohesion: 0.18
Nodes (9): CallResizeState, OverlayWindow, PlacementError, failed_window_update_closes_provider_views_and_requests_stop(), OverlayAction, OverlayAvailability, OverlayManager, Default (+1 more)

### Community 38 - "mod"
Cohesion: 0.13
Nodes (18): Activity, build(), Host, Mouse, Page, phone_enabled(), phone_needs_explicit_opt_in_even_for_legacy_profiles(), Placeholder (+10 more)

### Community 39 - "assist_overlay"
Cohesion: 0.17
Nodes (14): Rc, ActionQueue, CallResizeState, Content, Kind, OverlayWindow, Box, Cell (+6 more)

### Community 40 - "mascot"
Cohesion: 0.21
Nodes (7): Mascot, Mood, DrawCtx, Point, Rect, Self, Spring

### Community 41 - "profile"
Cohesion: 0.22
Nodes (18): a_directory_at_the_profile_path_fails_closed(), a_profile_created_during_staging_is_never_overwritten(), an_interrupted_write_leaves_no_final_profile_and_next_start_can_retry(), first_run_creates_profile_and_preserves_subsequent_user_choices(), initialize_at(), initialize_with(), profile_exists(), publish_without_replacing() (+10 more)

### Community 42 - "codex_server"
Cohesion: 0.19
Nodes (15): Process, Arc, AtomicBool, Drop, JoinHandle, Option, Path, PathBuf (+7 more)

### Community 43 - "notch_monitor"
Cohesion: 0.23
Nodes (19): a_saved_monitor_too_small_does_not_prevent_restart_on_primary(), cycle(), label(), main_monitor_selection_restores_saved_device_and_falls_back_without_changing_preference(), monitors(), next(), ordered(), place() (+11 more)

### Community 44 - "navigation"
Cohesion: 0.24
Nodes (13): feature_glyphs_are_static_native_icons(), glyph(), hit_testing_selects_every_tab_and_never_the_gaps_or_padding(), invalid_or_overflowing_coordinates_are_not_a_layout(), key_step(), Layout, panel(), Option (+5 more)

### Community 45 - "assist_overlay"
Cohesion: 0.21
Nodes (14): bounded_call(), call_action(), CallTarget, CallView, enumerate_monitors(), OverlayConfig, Prompt, QuestionForm (+6 more)

### Community 46 - "textfield"
Cohesion: 0.18
Nodes (6): Outcome, TextField, DrawCtx, Rect, String, TextStyle

### Community 47 - "worker"
Cohesion: 0.20
Nodes (12): F, drain_commands(), Peek, Color, HWND, Self, String, Vec (+4 more)

### Community 48 - "call_panel_size"
Cohesion: 0.23
Nodes (8): load(), parse(), Default, Option, Result, Self, save(), SizeDip

### Community 49 - "assist_glass"
Cohesion: 0.24
Nodes (11): DWMWINDOWATTRIBUTE, apply(), attributes(), contrast_setting(), high_contrast(), HighContrast, HWND, Option (+3 more)

### Community 50 - "codex_diagnostic"
Cohesion: 0.33
Nodes (10): connect(), Bridge, Fn, Option, PathBuf, Result, Snapshot, String (+2 more)

### Community 51 - "http"
Cohesion: 0.40
Nodes (9): encode(), fetch_bytes(), get_bytes(), get_json(), get_string(), Result, String, Value (+1 more)

### Community 52 - "assist_overlay"
Cohesion: 0.43
Nodes (6): apply_dashboard(), bounded(), bounded_dashboard(), DashboardView, readonly_content_is_bounded_and_keeps_the_selected_chat_visible(), same_input_context()

### Community 54 - "assist_overlay"
Cohesion: 0.40
Nodes (4): HBRUSH, PaintResources, Drop, HFONT

### Community 55 - "main"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

## Knowledge Gaps
- **332 isolated node(s):** `Self`, `OverlayManager`, `OverlayConfig`, `OverlayAvailability`, `StartWorker` (+327 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `apply_call()` connect `assist_overlay` to `assist_overlay`, `assist_overlay`, `assist_overlay`?**
  _High betweenness centrality (0.000) - this node is a cross-community bridge._
- **What connects `Self`, `OverlayManager`, `OverlayConfig` to the rest of the system?**
  _332 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `shell` be split into smaller, more focused modules?**
  _Cohesion score 0.05290072297654735 - nodes in this community are weakly interconnected._
- **Should `windows` be split into smaller, more focused modules?**
  _Cohesion score 0.0507048180096781 - nodes in this community are weakly interconnected._
- **Should `football` be split into smaller, more focused modules?**
  _Cohesion score 0.07192982456140351 - nodes in this community are weakly interconnected._
- **Should `gfx` be split into smaller, more focused modules?**
  _Cohesion score 0.06044303797468355 - nodes in this community are weakly interconnected._
- **Should `media` be split into smaller, more focused modules?**
  _Cohesion score 0.06846846846846846 - nodes in this community are weakly interconnected._