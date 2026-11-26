# Iteration 8 code map — real shared Codex server

Source status: refreshed after the explicit final source freeze, including the final test-fixture privacy correction in `src/assist.rs` (a synthetic identifier replaces the previous example). Graph structure is unchanged. The coordinator confirmed the repeated two-client live validation passed on the exact final release executable after that correction; its separate test/release rerun is noted below.

Snapshot date: 2026-11-26. The frozen Rust source was refreshed with `tools/Update-CodeGraph.ps1` and the already installed `graphifyy` 0.8.26 distribution. Input was only `src/`; the two new source modules are `codex_server.rs` and `codex_diagnostic.rs`. Graph extraction used local AST APIs, without an LLM backend, network command or application execution.

## Snapshot and portability

- 41 Rust files, 1,737 nodes, 4,215 edges and 40 communities.
- `graphify-out/SOURCE_SNAPSHOT.json` records SHA-256 for all 41 source files and zero additional LLM tokens. Every source hash matched after generation.
- Graph source paths are relative to the project. The public graph artifacts and review notes contain no personal absolute paths. `graphify-out/cache/` remains Git-ignored; no public interpreter-pointer file was produced.

The hashes cover local source bytes, not later edits or different checkout line endings. Historical maps remain unchanged. The graph supports navigation, not proof of dynamic dispatch, COM/FFI ownership or complete security coverage.

## Change in verified capability

Iteration 7 reported the compatible shared endpoint unavailable. In this iteration the coordinator explicitly started the official persistent daemon from the verified complete package, using the same CLI version `0.171.0-alpha.13.2`. Both first start and idempotent already-running behavior were observed. This supersedes the earlier endpoint-availability result; it does not connect the previously embedded Desktop server.

The coordinator then ran real headless NotchLite checks: server availability and a Bridge connection with ChatGPT authentication and available quota metadata. The final end-to-end test passed using one test chat and two prompts. The first materialized the chat; the second checked shared events after both clients were subscribed. Both clients observed the same chat, response, completion and available token-usage metadata; missed completion of the first turn was also recovered from history. These live results are separate from the unit tests below.

No test-chat identifier, prompt/response content, token quantity, account identifier or credential is included in this map. The test used the subscription path rather than a separately configured API-key provider. It does not establish that conversations already active in the Desktop GUI can be observed or controlled through this daemon. Those conversations were not migrated, restarted or connected.

## Source boundaries

| Boundary | Implementation and ownership |
|---|---|
| Explicit server start | `src/codex_server.rs:19` owns a bounded launcher worker. It invokes the official CLI with fixed arguments `app-server daemon start`; normal panel creation does not start it implicitly. The launcher has bounded output and a deadline, and validates the returned status, backend, versions and expected local socket. |
| Persistent daemon | The official daemon owns its package/state/process identity. NotchLite owns only the short-lived launcher and each client proxy. Launcher failure/cancellation and panel Drop never issue daemon stop, restart, update or bootstrap. A timeout can occur after the daemon has started; the error directs the user to check connectivity rather than terminating chats. |
| Complete CLI resolution | `src/codex_bridge/transport.rs:74` now prefers the official managed package, then standalone, before PATH/Desktop alias discovery. This addresses the observed Desktop-bundled bare executable that supported proxy but lacked a complete package for daemon installation/start. |
| Subscription boundary | `src/codex_bridge.rs` represents account readiness separately from transport connection. Only the ChatGPT account state is ready for the subscription-only workflow. Actual connection, account and quota errors remain distinct; missing data is not turned into a successful or zero-valued result. |
| Explicit new chat | The coordinator exposes a new-chat action for the shared server. `src/codex_bridge/protocol.rs:348` requests an OpenAI, read-only thread with user-reviewed untrusted approvals. Creation itself sends no prompt. The acknowledgement checks effective policy and labels unknown/mismatched policy explicitly instead of silently claiming read-only enforcement. |
| Existing actions | Responses carry current request/session identity; follow-up requires an idle, subscription-ready selected thread. UI creation, messages and responses remain explicit actions. Available thread token usage and account-limit metadata stay separate from shared process RAM/CPU. |
| Subscription and history recovery | `src/codex_bridge/protocol.rs:151` ignores loaded-list replies that predate a topology change. A thread becomes subscription-ready only after the resume acknowledgement. `src/codex_bridge/protocol.rs:184` then requests recent history to recover a missed completion. Event revisions reject stale history; a late idle notification permits at most two additional history reads, never a message replay. |
| Headless dispatch | `src/main.rs:35` calls `codex_diagnostic::run` before profile, COM, mutex or window initialization. Modes are mutually exclusive and arguments are checked. The older `--codex-check` remains a metadata-only presence check, not a connection test. |
| Connection diagnostic | `--codex-connect-check` connects a real Bridge, checks current transport/account state and waits for quota metadata, without creating a thread or starting a turn. Its quota-wait fallback rechecks the latest connection and account, so an old connected snapshot cannot mask a subsequent failure (`src/codex_diagnostic.rs:69`). |
| End-to-end diagnostic | `--codex-self-test` requires an explicit existing test workspace, deliberately creates a test chat and sends two prompts. The first materializes the chat; the second checks shared events after both clients are subscribed. This path is not part of normal startup and consumes subscription usage when explicitly invoked. |

## Public setup and practical commands

`tools/Setup-CodexSharedServer.ps1` is an explicitly invoked dependency helper. It downloads the pinned complete official archive, verifies the expected SHA-256 before extraction, checks archive entries, manifest, helper files and version, and uses a fresh extraction directory. Without `-Start` it does not start the server; `-Start` invokes the official daemon command. It does not change global PATH, perform login or configure autorun. The official daemon's first start may install its managed package/state under the existing user Codex home; this is documented rather than represented as merely extracting a file under the project.

The verified package is the exact alpha release, SHA-256 `6d3be3ea2e3f759f77f23b543ee882aae6433dc6a6cd260788c2e8793d616eac`. Full source/download provenance, subscription authentication, CLI attachment, explicit daemon stop and the new NotchLite headless commands are in `docs/CODEX-SHARED-SERVER.md`. No credential copying, private database inspection or Desktop restart is part of that procedure.

## Validation and limits

The repeated local logs after the final test-fixture identifier replacement record **107 tests passed, zero failed**, and a successful optimized GNU release build. Tests include bounded/strict launcher replies, explicit thread creation/policy acknowledgement, diagnostic privacy and state handling, resume readiness, stale loaded-list/history guards and bounded recovery after a late idle event, in addition to earlier UI/provider fixtures. The coordinator separately repeated the live shared-daemon and two-client end-to-end checks on the exact final release executable after that correction: both clients shared the same chat, response, completion and available token-usage metadata, history recovery passed, two messages were sent and no GUI was opened. The tested executable's SHA-256 is `139df501535225d6e193924272a697902d1e47b288520e3458d3d010a0511a53`. The source snapshot includes that correction.

This map task only refreshed AST artifacts, inspected source/summary logs and updated documentation. It did not start another daemon/client, submit another prompt, open a GUI or capture audio. No output containing runtime chat IDs, message bodies or account credentials was copied into public artifacts. Live GUI rendering, focus, secondary-monitor/DPI/hotplug behavior and Desktop-thread interoperability remain unverified. Audio AI stays disabled; call preparation is metadata-only. No new RAM/CPU/battery benchmark was performed, and historical measurements do not describe the shared daemon's added resource cost.
