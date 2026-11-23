# Iteration 5 code map — UI 0.1.5

Snapshot date: 2026-11-23. Refreshed with `tools/Update-CodeGraph.ps1` using the already installed `graphifyy` 0.8.26 distribution. Input was only the 26 Rust files in `src/`, including the new `navigation.rs`. The wrapper used local AST APIs without an LLM backend, network command, application launch, desktop interaction, installation or new dependency.

## Snapshot and portability

- `graphify-out/graph.json`: 1,019 nodes, 2,388 edges and 25 communities.
- `graphify-out/SOURCE_SNAPSHOT.json` records all 26 source SHA-256 values and zero additional LLM tokens. Every hash matched the local source after generation.
- The three generated public artifacts and this note contain no personal absolute paths. Graph source paths are relative to the project. `graphify-out/cache/` remains ignored by Git; no public interpreter-pointer file was created.

Hashes identify the local bytes read for this map. Later source edits or checkouts with different line endings need a fresh comparison. Historical iteration maps were preserved.

## Native navigation flow

| Boundary | Source trace |
|---|---|
| Geometry | `src/navigation.rs:23` computes a stack-only header layout for 1–12 tabs, checking finite coordinates, panel height and enough width for a readable status area. Unsupported layouts return `None`; the shell retains its legacy navigation. |
| Shared drawing/hit test | `src/shell.rs:598` draws the icon pill from `Layout`; `src/navigation.rs:43` provides each tab rectangle and `:55` tests the same rectangles. Padding, gaps and trailing edges are not tabs. Shell arrow/dot hit tests are disabled when the pill layout is active (`src/shell.rs:686`, `:702`). |
| Selection and hover | `src/shell.rs:809` routes a tab click through the existing `go_to` path, which completes the outgoing editor before changing pages (`:499`). Active tabs use the palette accent plus an underline; a visible footer names the current or hovered section. The hover title index is bounded (`:555`), and hover clears on page rebuild, leave, collapse and navigation. |
| Keyboard | `src/navigation.rs:61` maps local Ctrl+Tab and Ctrl+Shift+Tab to forward/backward steps. `src/shell.rs:936` accepts these only while expanded and outside shell/page editing. It uses the existing window messages, with no hook or global shortcut. |
| Native assets | `src/navigation.rs:70` maps existing feature IDs to static Segoe Fluent Icons glyphs. `src/main.rs:10` registers the module. There is no new icon image, runtime asset fetch, renderer, GPU device, worker or timer for the navigation. |

The panel remains 420×140 DIP with the existing body area and graphics resources. This increment replaces the expanded header navigation when it fits; it does not implement a larger panel or rebuild feature modules merely to navigate. The existing optional desktop clock and its lifecycle remain separate.

## Fourth appearance preset

`src/theme.rs:46` retains the existing preset values and adds Rose=3. Its persisted ID is `rose`; parsing, label, cycle and the atomic palette selector include it. The cycle is Original → Graphite → Midnight → Rose → Original. Missing or unknown IDs still select Original. The Rose palette uses a dark rose surface, pale text and pink accent; all color access remains through the existing shared getters.

The Settings regression test (`src/pages/settings.rs:340`) includes Rose while checking that unrelated preferences survive. Theme tests cover all four presets for persistence round trips, cycling and mathematical token contrast. These checks do not establish rendered accessibility or interaction correctness.

## Documentation and phone boundary

`docs/assets/navigation-preview.png` is a development/documentation schematic. It is not an application screenshot or a render captured from the binary, and is not included by the Rust source as an application asset.

`docs/PHONE-SHARE.md` records a proposed browser-based file-sharing flow for Android and iPhone, including unresolved authenticated-origin/transport requirements. It is outside the AST input. No Phone page, phone provider, QR session, listener/socket backend, actual pairing, transfer or mock successful connection was added by this UI increment. The navigation glyph table adds no Phone affordance. This map did not independently research or validate the web/platform claims in that design document.

## Evidence and limits

The new navigation module contains six pure tests for supported counts/status space, layout fallback, tab/gap hit tests, invalid geometry, shortcut mapping and existing-feature glyphs. This agent inspected source and regenerated the graph; it did not run Rust tests, build, launch NotchLite, use the desktop or open a socket. Test-suite/build results belong to the coordinator's separate validation.

The AST graph is a navigation aid and may omit or misinfer dynamic dispatch, Win32 reentrancy and FFI relationships. Independent review is recorded separately. No visual rendering, font availability, Windows key delivery, pointer interaction, DPI behavior, memory usage or CPU/battery impact was exercised here. Historical RAM results are not measurements of UI 0.1.5.
