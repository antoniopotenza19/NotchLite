# Iteration 7 code map — assistant panels 0.1.7

Snapshot date: 2026-11-26. The frozen source was mapped with `tools/Update-CodeGraph.ps1` using the already installed `graphifyy` 0.8.26 distribution. Extraction read only Rust code under `src/`, with local AST APIs and no LLM backend, network command, application window, audio operation or installation.

## Snapshot and portability

- 39 Rust source files, 1,680 nodes, 4,065 edges and 37 communities.
- `graphify-out/SOURCE_SNAPSHOT.json` records SHA-256 for every input and zero additional LLM tokens. All 39 hashes matched after generation.
- Graph source paths are project-relative. The three public graph artifacts and this map contain no personal absolute paths. `graphify-out/cache/` remains Git-ignored, and no public interpreter-pointer file was generated.

The hashes identify the local file bytes. Later source edits or checkouts with different line endings require a fresh comparison. Historical maps were preserved. AST links are navigation evidence, not proof of dynamic dispatch, Win32 reentrancy, COM ownership or runtime correctness.

## Current capability boundary

The left panel is a Codex thread dashboard, not a generic new AI-chat provider. It has native controls for thread selection, status, available usage data, detail, up to three non-secret questions and an explicit idle-thread follow-up. The provider requires a compatible, already running shared local Codex control endpoint and the official CLI proxy capability. It does not scrape desktop windows or silently substitute a newly started daemon.

The coordinator's `--codex-check` validation reported the canonical shared endpoint unavailable. That helper returns before settings/profile initialization, COM, mutex or window creation (`src/main.rs:34`), and its `check_local` path only checks local endpoint/CLI metadata (`src/codex_bridge.rs:147`). No proxy was connected and no daemon or GUI app was started for that check. Access to the user's current desktop Codex conversations has therefore **not been demonstrated**. Whether those conversations can participate in an appropriate shared endpoint remains a separate product/capability decision; the UI must continue to show unavailable rather than pretend it is connected.

The right panel is call preparation and possible-call metadata only. In this subscription-only release, confirmation cannot start capture: `AudioMonitor::confirm` returns the disabled reason, `poll_audio` returns no chunks, and the coordinator always supplies `can_listen=false` and `listening=false`. There is no PCM acquisition, microphone recording, transcription or AI-response suggestion pipeline. Visible labels say that audio acquisition is absent.

## Source flow

| Boundary | Implementation |
|---|---|
| Enable and settings | `src/assist.rs:85` decodes `codex_dashboard` and `call_panel` as exact opt-ins; absent values stay off. `assistant_monitor` is optional. The Settings monitor selector enumerates secondary devices only on its explicit click (`src/pages/settings.rs:194`), not while drawing. |
| Inert UI construction | `OverlayManager::new` retains empty optional window/provider-data state. Native windows, GDI fonts/brushes and the action queue are created lazily. Drawing never starts a provider, samples processes or performs file/network I/O. |
| Placement | `src/assist_layout.rs:21` excludes primary monitors and preserves an explicitly selected device. An absent secondary becomes a waiting state; ambiguity requires selection. Pure geometry places the dashboard at the work area's top-left and the call panel top-right, with scaled margins, a non-overlap gap and bounded arithmetic for negative coordinates. Insufficient space does not cause a primary-monitor fallback. |
| DPI and hotplug | Overlay display/work-area/DPI messages hide the affected window, mark topology dirty and wake the main loop. `Assistant::tick` and `actions` consume that signal before the availability guard (`src/assist.rs:163`, `:187`). The coordinator updates placement and worker availability together. A newly visible call panel also requests this coordinated refresh. No new periodic monitor-enumeration timer was added. |
| Rendering and input | `src/assist_overlay.rs` uses native EDIT/COMBO/button controls and GDI, with cached fonts/brushes updated for DPI/palette changes. It creates no new GPU device or renderer. Updates use non-activating show/position calls, not foreground/focus-stealing calls. Native dialog navigation is routed from the shared message loop. |
| Attention and drafts | New-message, question and completed-thread attention comes from coordinator state; Domanda is yellow, Conclusa green and other message attention blue. There is no flashing animation. Input remains only within the same thread/request/prompt identity; metrics or status refreshes do not erase it. Repeated text is not used as a deduplication rule that could prevent retrying a failed action. |
| Question and action boundary | Question answers carry the exact `AnswerKey` and prompt IDs. Option descriptions remain visible in the scrollable detail, while the COMBO answer value remains the exact option label. Oversized/unsupported forms cannot submit a partial answer; secret questions and approvals direct the user to Codex. UI callbacks enqueue typed, bounded actions; the bridge validates current request/session identity and idle-thread state again. Untrusted detail/status text is displayed as text, not interpreted as commands. |
| Close and resources | Closing the call window queues Stop and suppresses that candidate's automatic reappearance. Stop survives a full action queue. `WM_NCDESTROY` clears borrowed window pointers; owned controls are destroyed before font/brush state. Global UI shutdown works outside the main App borrow. Closing the dashboard updates coordinator availability before stopping/persisting the flag. |

## Provider, metrics and ownership

`Bridge::start` (`src/codex_bridge.rs:88`) first resolves the compatible local endpoint and CLI. Only then can the opt-in provider own a worker and `codex app-server proxy --sock` child; it does not start an upstream daemon. Bounded command/frame/write paths and the owned proxy shutdown are in `src/codex_bridge/transport.rs`. That live proxy path was not exercised against the user's daemon during this iteration.

Token usage belongs to the selected thread only when the API supplies it. Rate-limit information is account/window data when supplied, not a fabricated remaining token count. `src/assist_metrics.rs:27` reports NotchLite and Codex process working set/private bytes/CPU separately; shared process resources are not attributed to individual chats. Missing or partial measurements are labelled accordingly. Metrics and call-metadata workers are separately owned and stopped when unavailable or disabled; the UI layer itself adds no worker or timer.

Coordinator error rendering prioritizes an actual bridge failure over an earlier queued-action note (`src/assist.rs:308`). Connection/action status is also retained in the readable detail area, so a second-line failure is not hidden by a single-line caption. A queued action is not represented as server-confirmed success.

## Validation and remaining evidence

The coordinator's final CLI logs record **95 tests passed, zero failed**, and a successful optimized GNU release build in **25.70 seconds** after the final option-description/secret-form regression was added. The assistant component tests cover pure monitor selection/geometry, an inert disabled manager, queue pressure/Stop, capability gating, bounded content, localized attention tones, input context, one-shot topology signals and deferred call layout. No test creates these overlay windows. Protocol and provider fixtures remain distinct from a live connection to existing GUI chats.

This map task only regenerated AST artifacts and inspected source/log evidence. It did not run another build, connect to a Codex endpoint, start a daemon/proxy, show a window, inspect the desktop or capture audio. Actual two-monitor rendering, native focus/input, DPI/hotplug transitions, deep-link opening, subscription endpoint interoperability and runtime RAM/CPU/battery impact still need their own permitted evidence. Earlier RAM reports are historical, not measurements of 0.1.7.
