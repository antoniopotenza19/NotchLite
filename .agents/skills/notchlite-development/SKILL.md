---
name: notchlite-development
description: Coordinate NotchLite feature work and refactors with current code mapping, small complete changes, independent review where useful, and scoped validation. Use only for this repository's development; small low-impact fixes do not require a team.
---

# NotchLite development

Read [the project team](../../../team/DEVELOPMENT.md) when a change needs several roles. Assign the actual files and outcomes, keep work shared, and use only roles that improve this task. Existing user authorization and host permissions remain the authority; this workflow adds no approval step.

## Orient

Use a current Graphify graph if present, checking its source snapshot. Refresh with tools/Update-CodeGraph.ps1 only when mapping helps and Graphify is already available. The existing global Graphify skill may provide navigation; do not duplicate or install it. If absent, read the relevant files and callers directly. The AST graph is a map, not proof of runtime behavior.

## Make the smallest complete change

Apply the adapted Ponytail method here. The [original pinned skill](references/ponytail-upstream.md) is retained for provenance; its whole-session activation and broad trigger do not apply outside this project.

Read the changed flow and every caller of a signature or shared helper that will change. Solve the full user request with the least new code: reuse this project's helpers, then native/standard APIs, then an installed dependency if needed. Preserve validation, error handling, security, accessibility and platform behavior when moving or merging code. Avoid speculative options, wrappers and frameworks.

A nontrivial logic change should leave a meaningful regression check or an explicit account of the runtime check that could not be performed. Trivial edits need no invented tests. Do not add an abstraction merely to make a function shorter.

## Review and validate

For material changes, an independent reviewer can use [ponytail-review](../ponytail-review/SKILL.md) with the actual source, callers and raw evidence. Do not pass a desired verdict. Small fixes may use a direct self-check, labelled accurately.

Validate changed behavior with allowed builds and synthetic inputs. Do not equate a passing unit test with a Windows interaction check or a security reproduction. Report measured RAM/CPU with its module profile and scenario. Keep untested behavior visible without declaring the whole product safe or complete.

For source provenance, see references/SOURCES.json and references/LICENSE.txt. No source from these guidance files is linked into the application.
