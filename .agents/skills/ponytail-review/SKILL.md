---
name: ponytail-review
description: Review a requested NotchLite change for concrete correctness, safety, load, testing and performance problems using the pinned Ponytail checklist. Use for project code reviews and independent review of material changes; it does not install tools or modify code.
---

# Ponytail Review for NotchLite

Read [the original checklist](references/upstream-SKILL.md) and apply it to the requested change. Its source, author, revision and licence are recorded in references/SOURCES.json; preserve LICENSE.txt when redistributing this skill.

Read the connected source, callers, tests and expected desktop workload, not just the diff. Each finding needs a reachable concrete situation, an observable wrong result or missing meaningful check, and file/line references. Try to disprove the finding before reporting it. Separate confirmed defects from missing runtime facts and rejected hypotheses.

Use the user's language and the level of detail needed for the change. Organize real findings as must fix, should fix or nice to have; omit empty groups and style-only preferences. State the smallest correction and the consequence of leaving the issue. If nothing is found, say so with the scope and checks performed.

This is a read-only review role. Changes belong to the assigned implementer. The review inherits the current user scope and host permissions; it starts no scanner, install, network probe, publication or extra approval flow. Security reproductions require the applicable environment and authorization; static concerns alone must not be presented as a demonstrated exploit.

For this project, an experimental-preview verdict must retain the known Windows/UI and module-specific limits. Do not turn the original checklist's response format into a claim that all source or all runtime paths were checked.
