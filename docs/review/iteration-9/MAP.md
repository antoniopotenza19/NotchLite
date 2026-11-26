# Iteration 9 code map — local call transcription

Snapshot date: 2026-11-26. The source was refreshed after the explicit final
freeze, including Stop keeping the text panel visible, fatal STT errors
revoking capture before worker cleanup, and rearming a later observed
communications occurrence that reuses an existing process/session.

## Current graph

- 45 Rust source files; SHA-256 for every file is recorded in
  `graphify-out/SOURCE_SNAPSHOT.json` and checked against the current source.
- 1,945 nodes, 4,815 edges and 42 communities; the report displays 41
  communities and omits one thin community.
- Generated with the already installed Graphify 0.8.26 distribution through
  `tools/Update-CodeGraph.ps1`, using local AST APIs only.
- Zero additional LLM tokens. The extraction did not run the application,
  start audio, invoke a model, install packages or use a network backend.
- Source paths in the public map are relative. The cache remains ignored by
  Git. No local interpreter pointer is distributed.

The AST graph supports navigation. It does not establish dynamic dispatch,
COM ownership, correctness of native capture or complete security coverage.
Its hashes describe the local source bytes; later edits or different checkout
line endings require another comparison.

## Confirmed flow in source

`assist` coordinates the native overlay and the metadata monitor. A capture
starts only after an explicit action for a selected session. `call_audio`
assigns a generation synchronously; `call_audio/capture` activates process
loopback for that process and its children, with no microphone or global
audio fallback. PCM enters a bounded queue and is revoked on Stop, identity
loss, error or overflow.

`call_session` groups PCM into speech segments, pads short acknowledgements,
tracks forced-cut overlap and rejects results from another generation.
`call_stt` lazily launches the verified CPU whisper.cpp runtime for a segment.
Only a single active job and bounded latest pending/result slots are kept.
Temporary audio/text files are confined to an owned private directory and
cleaned up by the worker. The optional model is not loaded while idle.

Transcript and suggestions reach native read-only controls in
`assist_overlay`. Stop revokes acquisition before waiting for IPC/STT cleanup
and keeps the text panel visible. Close also dismisses the panel. A fatal
STT result now propagates to the controller, which stops capture; it does not
restart listening automatically.

When the metadata monitor observes a communications-off interval, the prior
Ignore state is cleared even if the process/session remains alive. The
controller and overlay clear their corresponding dismissal/suppression
state. A later observed occurrence can be suggested again, but it still
requires explicit confirmation to start capture. End/Begin notifications
between refreshes can remain indistinguishable; this heuristic does not
guarantee detection of every incoming call and retains manual selection.

`call_coach` is a separate optional bridge for an assistant participating in
an active chat. It exposes only bounded transcript/suggestion operations on
authenticated IPv4 localhost. It does not invoke a model, execute tools or
reuse host credentials. An inactive assistant chat does not generate replies.
The existing shared Codex dashboard bridge remains a separate integration.

## Verification status

The current local test summary records **131 passed, zero failed**, followed
by a successful optimized GNU release build in **29.28 seconds**. The final
executable SHA-256 supplied by the coordinator is
`96cb6d39c1c083c8db41f609efd58bc9b2d2267903e5714e06c2db49d39d5598`.
The tests cover
capture identity/generation and bounded synthetic PCM, speech segmentation,
short utterances, overlap handling, STT parsing/queue cancellation, local IPC
authentication/replay/Stop, the native controller's Stop/Close distinction,
and metadata/overlay rearming without automatic capture.
Those tests do not activate audio devices or open GUI windows.

The coordinator tested that exact final executable with a six-second public
PCM16 mono 16 kHz WAV: 96,000 samples produced nonempty English transcription
in 281 ms. The tiny model has shown word/name errors; this is a file-only
functionality check, not proof of call accuracy or Italian quality. The
resulting public-fixture text then passed through `--call-coach-check` and
`--call-await` into this active development chat. The coordinator formulated
a short response and submitted UTF-8 JSON through `--call-suggest` using an
absolute local input path. The client reported `accepted`; the waiting
product process reported `suggestionReceived: true`, `audioCaptured: false`
and `guiOpened: false`. The owned pointer file was removed after Stop. This
verifies the optional bridge with an actively participating assistant, not
an autonomous model running after a chat becomes inactive. Raw summaries
and build evidence remain separate from the AST graph.

No real call, microphone, native capture, rendered overlay, focus/DPI/hotplug
or full GUI resource benchmark was performed by the mapper. Earlier
performance measurements must not be attributed to the new call pipeline.
