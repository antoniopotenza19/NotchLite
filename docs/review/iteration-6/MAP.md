# Iteration 6 code map — phone LAN preview 0.1.6

Snapshot date: 2026-11-23. The current graph was refreshed with `tools/Update-CodeGraph.ps1` and the already installed `graphifyy` 0.8.26 distribution. Input was only `src/` Rust code: 30 files, including `phone.rs`, `phone/wire.rs`, `pages/phone.rs` and `phone_qr.rs`. Graph extraction used local AST APIs, no LLM backend or network operation. Its artifacts record zero additional LLM tokens.

## Snapshot and portability

- `graphify-out/graph.json`: 1,224 nodes, 2,927 edges and 29 communities.
- All 30 source SHA-256 values matched `graphify-out/SOURCE_SNAPSHOT.json` after the refresh.
- Graph source paths are project-relative. The three public graph artifacts and this map contain no personal absolute paths. `graphify-out/cache/` is Git-ignored; no public interpreter-pointer file was generated.

These hashes cover the local file bytes, not subsequent edits or checkouts with different line endings. Historical iteration maps were preserved. The AST graph remains a navigation aid; it does not prove runtime reachability, correct FFI ownership or security.

## Product boundaries and data flow

| Boundary | Implementation |
|---|---|
| Explicit enable | `src/pages/mod.rs:178` defines the dedicated `phone` key; only `phone=1` enables it, including for old profiles. The builder separately adds the page before Settings and reuses surviving page instances. The inert `PhonePage::new` (`src/pages/phone.rs:33`) creates no listener or worker. |
| Start and display | `src/pages/phone.rs:84` selects a locally enumerated IPv4 adapter and starts the service only after the Start action. Multiple candidates require a network choice; there is no UDP route probe. `src/phone.rs:124` accepts only a selected active RFC1918 address and binds that address with an ephemeral port, never the wildcard interface. |
| UI work | Drawing reads a cached snapshot. The existing shell tick refreshes status; `PhoneService::snapshot` uses `try_lock` and a cached fallback instead of waiting for a disk publish. UI decisions also return a busy error rather than block on that mutex. File dialogs/start/open/stat are action paths, not drawing paths. |
| QR window | `src/phone_qr.rs:40` uses `qrcodegen` and a separately owned GDI window with a stable boxed state and explicit cleanup. It adds no GPU renderer or timer. Quiet-zone and integer-module geometry are pure helpers; physical scanning was not tested. |
| Worker ownership | `src/phone.rs:287` runs one listener thread and at most four connection workers, all owned by the optional service. Socket read/write timeouts must configure successfully; the initial handshake/headers have an absolute 10-second deadline, and each body/response a fixed 120-second deadline. A transfer cancellation also interrupts a socket waiting for bytes. |
| Rendering resources | The existing notch renderer, text-format cache, palettes and animation loop are reused. The backend does no GPU work. The enabled provider does add TLS/session/thread resources; their cost is not covered by earlier idle measurements. |

## Pairing and session contract

The QR URI contains the selected IPv4/port, SHA-256 of the exact DER certificate and a 256-bit random one-use secret. Native clients must verify the pinned identity during TLS before sending the secret; this is not a browser endpoint. The generated ECDSA identity has IP SAN, server-auth EKU, digital-signature usage, a five-minute past clock-skew allowance and a 24-hour validity period.

`POST /v1/pair` consumes the secret and creates a random request ID, poll token and comparison code. The PC approves or denies the exact pending ID. The monotonic deadline remains 120 seconds from service start, including approval; it is not extended by scanning late. Polling only returns a session bearer after approval. Secret, poll token and session bearer have separate purposes and are held in memory rather than saved as a reconnect identity.

The preview permits one volatile device/session. Closing a TCP connection or losing Wi-Fi does not itself revoke that session; PC Stop, disabling the module or dropping the service does. Restarting creates a new identity/secret/session. A successful pairing does not imply Bluetooth, notifications, calls or remote desktop support.

## File transfer and cancellation

The authenticated HTTP/1.1 protocol uses one request per connection and `Connection: close`. Headers are capped at 16 KiB, JSON at 8 KiB, and a file at 256 MiB. Transfer-Encoding, duplicate headers, ambiguous lengths, browser Origin and unsupported framing are rejected. Raw uploads require the exact offered Content-Length after PC acceptance. There is at most one current incoming and one outgoing offer; transfers stream through a 64 KiB buffer instead of loading the file into RAM.

`src/phone/wire.rs:299` atomically claims an accepted upload before opening its staging file. Incoming names are validated as single Windows file names, including reserved device names. The receive root and ancestors reject reparse directories and retain directory handles. Each accepted transfer uses its own generated directory and create-new partial file; completion publishes with `MoveFileExW` without replacement. Failure/cancellation removes only the owned partial and empty owned directory; completed received files are retained.

Outgoing metadata and bytes come from the same explicitly selected file handle, opened without write/delete sharing. A completed download can be retried from that handle until the client acknowledges; the acknowledgement accepts either an empty body or a bounded empty JSON object. Replacing a completed offer makes its previous ID obsolete. Progress counts reflect bytes successfully written by the streaming loop; they are not proof that a phone has persisted the download.

Stop sets the revocation flag, interrupts active sockets and waits at most approximately 900 ms for its listener worker. Normal worker completion releases owned resources. An OS-blocked disk operation may outlive that wait; its owned state/handles remain alive until the operation returns. Stop prevents a publish that has not started, but cannot undo an already-entered Windows rename syscall. This is an explicit limit, not a promise of kernel-resource release within 900 ms.

## Validation and local build tool

The backend author ran 16 tests selected by `phone::` (including two Phone page tests). They passed. Real TLS connections were confined to a cfg-test loopback listener on `127.0.0.1`: wrong identity rejection, QR pin derivation, matching request/code, pre-consent authorization denial, approval/expiry/denial, exact upload bytes, download retry and acknowledgement, cancellation during a partial upload, partial cleanup, and Stop interrupting an idle handshake. Pure/synthetic checks cover framing, size limits, names, bearer comparison, snapshot behavior and no-overwrite/owned cleanup. These tests did not contact a phone or LAN adapter. The coordinator separately reported all 59 Rust tests passing; final release build results belong to its build validation.

Ring's C build used a local-only [LLVM-MinGW 20261117 release](https://github.com/mstorsjo/llvm-mingw/releases/tag/20261117), asset `llvm-mingw-20261117-ucrt-x86_64.zip`, [official download](https://github.com/mstorsjo/llvm-mingw/releases/download/20261117/llvm-mingw-20261117-ucrt-x86_64.zip). Its SHA-256 matched the GitHub release asset digest: `4d002e330a0f703625aa5ccbc37e96f8f824413e163bfc7336774aabd2c3a412`. It was extracted under ignored local work storage, with no global installation or persistent PATH change. This is a build tool, not an app runtime component.

No NotchLite GUI was launched and no desktop, firewall or OS setting was changed. Native Android/iPhone trust behavior, camera scanning, actual file-picker/export flows, private-network restrictions, DPI/visual rendering and RAM/CPU/battery costs still require their respective real-device/runtime evidence. This map does not substitute for the separate independent security review or companion validation.
