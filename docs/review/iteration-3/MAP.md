# Iteration 3 code map

Snapshot date: 2026-11-23. Refreshed with `tools/Update-CodeGraph.ps1` and the already installed Graphify distribution `graphifyy` 0.8.26. Input was only the 24 Rust files in `src/`. The wrapper used local AST extraction, with no application launch, desktop interaction, LLM backend, network command, installation or new dependency.

## Snapshot and portability

- Current generated map: `graphify-out/graph.json`, 943 nodes, 2,219 edges, 28 communities.
- Every source SHA-256 matched `graphify-out/SOURCE_SNAPSHOT.json` after generation. That file records all 24 source hashes and zero additional LLM tokens.
- The three generated public artifacts contain no personal absolute paths; graph source paths are project-relative. `graphify-out/cache/` remains ignored by Git. No interpreter-pointer file was created in the public output directory.

The source hashes identify the local files read for this map, rather than promising that later edits or a checkout with different line endings will remain covered. Historical architecture snapshots were preserved.

## Appearance flow

1. `src/pages/settings.rs:56` cycles the `Appearance` row through Original, Graphite and Midnight using the fresh settings map. A successful save sends `ReloadSettings` (`src/pages/settings.rs:175`); failure retains a visible error state. Settings actions and pointer-entry refresh use the shared fallible reader `store::try_read_kv` (`src/store.rs:45`), preserving the previous map on read errors.
2. `src/theme.rs:42` defines the stable `appearance` key. Parsing an absent or unknown value selects Original. The three static palettes feed runtime color getters; `AtomicU8` selects the active preset (`src/theme.rs:119`). This path introduces no worker, lock, per-draw allocation or file access.
3. `src/shell.rs:272` reads settings successfully before changing any active state. A read error returns immediately, preserving palette, feature selection and pages instead of treating the error as an empty file that could enable modules. Initial startup propagates the read failure. A valid empty file retains its normal default-setting meaning. After a successful read, the existing page-rebuild guard still depends on the disabled-feature list, not appearance. An appearance-only change therefore retains page instances and their running state. When features change, `src/pages/mod.rs:181` reuses surviving pages by ID. The existing mascot timer synchronization still runs; the palette adds no timer.
4. `src/gfx.rs:163` caches text formats without storing color in the cache key. `src/gfx.rs:359` updates the solid brush for each draw. Palette changes need no font/image cache flush or graphics-device recreation.
5. `src/pages/calendar.rs:363` now consistently draws selected day labels with background-colored ink over either the accent or text-colored selection surface. This avoids the previous selected-weekday muted-ink contrast mismatch. The pure contrast checks in `src/theme.rs:185` cover text-token alpha compositing on panel/hover backgrounds and background ink on selected-control surfaces for all three dark presets.

The tray-setting path (`src/shell.rs:307`) also uses the fallible shared reader before changing a key and reloads only after a successful write. A read failure therefore cannot replace saved appearance or disabled-feature selections with a default map through this caller.

## Limits

This is an AST/navigation refresh and targeted source trace, separate from the independent code/security review. AST inference can misidentify same-named calls: the generated report's inferred link between the theme test's `contrast()` and football's `luminance()` is not a verified runtime dependency. COM/FFI, dynamic page dispatch and Windows message timing require direct source or runtime evidence.

The presets style NotchLite only. No light preset or Windows-wide theme change is implemented by this increment. Existing queued `Peek` values retain the concrete accent captured when created (`src/worker.rs:46`); changing the active palette does not recolor an already queued notification. No GUI, DPI, visual contrast rendering, worker continuity or RAM behavior was tested by this map task. The coordinator reported 27 passing unit tests; this agent did not rerun that suite or the release build.
