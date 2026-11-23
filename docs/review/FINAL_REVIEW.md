# Independent final review

What this change does: The patches restrict dragged links to web URLs, stop the weather worker while it waits, and repair several Windows input and shutdown paths. They keep the existing native Ada-based application and add small helpers rather than a new framework.

This review covers the experimental preview, not a stable or fully audited product. It assumes one desktop user, one application process, optional weather/media modules, and settings changed through the app. I read all of shell.rs, droptarget.rs and pages/weather.rs, their changes against the upstream snapshot, and the connected page, editor, HTTP and worker paths. I changed no application code and ran no target, exploit or scanner.

## Checked source snapshot

| File | Lines | SHA-256 |
|---|---:|---|
| src/shell.rs | 1239 | 5f53a38048852777327bb27fbab4e50089511d6bb77e2907bd95fddccf2b8c70 |
| src/droptarget.rs | 443 | a76f11d0f05fd13f19403f88ff84ee0d8b2d32fa55693cff9cdf719cc483c5c2 |
| src/pages/weather.rs | 482 | 9fb2a37ca30c4df7e829c5b0e32e6ca2a9649fd55ea35f676a5d365832610b57 |

Method: the read-only Ponytail review checklist from DietrichGebert/ponytail, revision ebd8654c2c627f721eb8f9c0d1bd4090a29f9e10, MIT. This is a review method, not a new app dependency. No social account data belongs in the published review.

## Should fix

1. **The page-change and Exit fixes still need a Windows regression check** (src/shell.rs:400-410, 750-765, 843-875, 1039-1049).
   - **What this is:** Changing pages now finishes the outgoing editor. Exit now handles the nested destruction message before borrowing application state.
   - **Problem:** The new shell tests cover Unicode decoding only. They do not prove that a Todo draft is saved once when Next is clicked, or that selecting Exit ends the process and allows an immediate restart.
   - **Fix:** Record those two concrete Windows checks in an isolated test setup, or add a small routing test with a fake Page where practical. Keep the manual check pending when the UI cannot be targeted; do not claim it passed.
   - **If we skip it:** These two user paths remain supported by source reasoning only. The preview must keep that limitation visible.

2. **Music can keep scheduling frames while the overlay is hidden** (src/shell.rs:228-230, 352-377, 413-432, 435-543; src/pages/media.rs:1052-1053).
   - **What this is:** The shell hides its window during fullscreen use. The music module requests animated frames while a track plays.
   - **Problem:** With music enabled and playing, hidden_for_fullscreen changes window visibility but does not enter the decision to keep the animation timer running or to draw frames. Work remains scheduled for a hidden overlay.
   - **Fix:** Suspend ambient animation/rendering while hidden and request a fresh frame when the window returns. Preserve the underlying playback and timer state; no new scheduler framework is needed.
   - **If we skip it:** The low-power idle result does not describe this fullscreen-with-music path. The extra CPU/GPU cost is not measured in this review. This is inherited behavior, not a regression introduced by the patches.

## Corrections supported by the source

- URL checks now apply to the registered URL clipboard format, the plain-text fallback, and save_url_shortcut itself (droptarget.rs:200-210, 220-237, 347-350). Validation occurs before directory creation. The code rejects literal controls, internal whitespace, backslashes and non-http(s) schemes. This closes the reviewed direct path to writing a non-web shortcut; it is not a claim to implement every URL syntax rule.
- WeatherPage::drop sends a stop signal, and the page builder drops disabled pages that were not reused (weather.rs:104-147; pages/mod.rs:180-211). The sleeping worker observes the stop through recv_timeout. An already-started fetch remains cooperative and is explicitly documented as such.
- WM_DESTROY cleanup runs before try_borrow_mut, including reentry from DestroyWindow in the tray menu. Moving this small branch is proportionate to the defect.
- UTF-16 input state is reset on a new editor session and when the outgoing page is finished. The TextField already stores Unicode characters and UTF-8 safely; it did not need a rewrite.
- go_to calls the outgoing on_focus_lost hook before changing current. Todo and Calendar already implement that hook, so reusing it avoids duplicate save logic.
- set_startup no longer deletes the unrelated legacy YESDEV value.
- The new weather helper is justified by the cancellation tests: it separates the wait/fetch control flow without introducing a dependency.

## Counterevidence and publication notes

The normal tray-to-Settings stale-state claim was not confirmed by this review. Settings handles Mouse::Leave by clearing inside, then rereads settings.txt on its first Mouse::Move after entry (settings.rs:205-224). A claim that the ordinary pointer trip through the tray always leaves Settings stale omits that path. A narrower case requires its own evidence.

The portable instructions were found to advertise right-click on the notch itself, while the custom menu is called from WM_APP_TRAY only (shell.rs:1090-1093). The smallest correction is documentation: use the tray icon for Exit. This was sent to the coordinator for packaging; no new UI behavior is required.

There are 15 unit tests in the reviewed sources. The coordinator reports that the normal synthetic-input suite passed; this reviewer did not rerun it or independently observe that execution. The reviewed new tests address URL rejection, Unicode decoding and weather cancellation. No new must-fix code defect was demonstrated in these patches.

Verdict: Ship.

This verdict is for an explicitly experimental preview with the stated limits and corrected Exit instructions. It is not a security certification, a declaration of feature parity, or a claim that every module meets the RAM budget.

Not checked: live Windows UI, tray Exit/restart, page-change persistence on screen, registry behavior, real network endpoints, malicious OLE objects, long-run resource growth, DPI/multi-monitor behavior, and the final rebuilt binary.
