# Iteration 4 code map

Snapshot date: 2026-11-23. Refreshed with `tools/Update-CodeGraph.ps1` using the already installed `graphifyy` 0.8.26 distribution. Input was only the 25 Rust files in `src/`. Local AST extraction introduced no application launch, desktop interaction, LLM backend, network command, installation or dependency.

## Snapshot and portability

- `graphify-out/graph.json`: 1,001 nodes, 2,347 edges and 24 communities.
- `graphify-out/SOURCE_SNAPSHOT.json` records SHA-256 for all 25 input files, with zero additional LLM tokens. All 25 hashes matched the working tree after generation.
- The three generated public artifacts and this note contain no personal absolute paths. Source-file paths in the graph are relative to the project. `graphify-out/cache/` remains ignored by Git; no public interpreter-pointer file was created.

Source hashes identify the local file bytes read for this map. Later source edits or a checkout with different line endings require a fresh comparison. Earlier iteration maps and historical reviews were preserved.

## Desktop clock lifecycle

| Boundary | Source trace |
|---|---|
| Opt-in | `src/desktop_clock.rs:29` enables only `desktop_clock=1`; missing values leave it off. `src/profile.rs:15` explicitly writes `desktop_clock=0` for new profiles. `src/pages/settings.rs:104` adds a flag row separate from the enabled-page list. |
| Reload | `src/shell.rs:284` completes the fallible settings read before changing active state, applies the palette and calls `desktop_clock::sync`. A clock creation/refresh failure is reported without preventing the notch's remaining settings from being applied. |
| Shared timing | `src/shell.rs:820` passes its existing local-time sample to `desktop_clock::tick`. The existing one-second `TIMER_TICK` drives both the notch and the clock; no clock timer or worker is added. The clock uses a full date/hour/minute key and invalidates text only when it changes, becomes visible or changes layout. |
| Native resources | `src/desktop_clock.rs:241` creates a separate optional Win32 window and cached GDI fonts. It does not call `Gfx::new`, create a new GPU device or renderer, or launch another process. Font replacement is tied to DPI changes; successful region assignment transfers region ownership to Windows. |
| Paint | `src/desktop_clock.rs:384` paints with GDI and the current theme tokens, restoring selected objects and drawing settings. Its dedicated window procedure pairs `BeginPaint`/`EndPaint`; it does not access the shell's borrowed `APP` state. |
| Ownership and reentrancy | The clock keeps its own thread-local manager and stable boxed window state. `WM_NCDESTROY` marks the window destroyed and clears the borrowed `GWLP_USERDATA` pointer. `shutdown` can defer removal until a manager borrow is released; removal happens before `DestroyWindow` callbacks. Clock/font `Drop` paths release their owned resources. |
| App exit | The notch's `WM_DESTROY` calls clock shutdown before posting quit (`src/shell.rs:1165`), without needing another `APP` borrow. Shutdown also runs after the shared message loop exits (`src/shell.rs:1359`). The clock procedure itself does not post app quit. |

The page-rebuild guard still depends on changes to the disabled-feature list. Toggling the clock or palette alone does not rebuild pages, reset a running Focus session or discard graphics caches. The clock's GDI path is distinct from the notch's existing Direct2D renderer.

## Visibility and positioning limits

The clock is a normal optional application window with no-activation/tool-window/click-through styles. Each existing shell tick checks the foreground window class; it shows only for `Progman` or `WorkerW` and hides otherwise. It is **not pinned or parented into the Windows desktop**, does not modify Explorer's desktop hierarchy and does not provide wallpaper or dock integration. The class check is a deliberately limited visibility heuristic, not a general proof that every desktop configuration is recognized. Visibility can lag the foreground change by the existing timer interval.

Layout uses the primary desktop work area and current window DPI. It fits the calculated rectangle inside that area and refreshes placement/fonts when layout changes. The arithmetic helper accepts negative and extreme coordinates in unit-test inputs; this does not establish multi-monitor placement support, dragging or runtime DPI correctness.

## Evidence and limits

The new module contains six unit tests covering opt-in parsing, deferred/idempotent shutdown without a window, 24-hour formatting, date-sensitive redraw keys, the limited foreground-class policy and bounded layout arithmetic. Settings also has a regression test preserving the preset and disabled features while toggling the flag. This map task inspected those tests but did not run Rust tests, a build or the application; execution results belong to the coordinator's validation.

The AST graph supports navigation, not proof of correct FFI ownership, safe reentrancy or runtime reachability. Independent review remains separate. No GUI, desktop visibility, click-through behavior, actual GDI resource release, font rendering, DPI change, memory usage or battery impact was exercised here. Zero added GPU devices/workers/timers describes the source implementation, not a measured RAM or CPU result.
