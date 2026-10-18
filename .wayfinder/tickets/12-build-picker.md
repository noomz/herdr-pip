---
title: Build: picker
type: task
status: closed
assignee: noomz
blocked_by: [10-build-skeleton.md]
---

## Question

Attention-first then most-recent list excluding the focused pane; row format, type-to-filter, `j`/`k`/arrows, click, wheel, snapshot refresh on re-show, empty state. Choosing a source hands off to the reopen helper (or in-place until it exists). Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md).

## Progress (2026-10-18; ticket stays claimed until the live smoke passes)

Built, not yet closed. `cargo test` 17 pass, clippy clean, release build done. Not committed.

- `src/picker.rs`: candidates are every snapshot pane except the focused one and the peek's own `HERDR_PANE_ID`. They are ordered by herdr's own attention order (blocked, done, working, idle, plain shell), then newest `agents[].state_change_seq`; shells keep snapshot order. Row: `!` blocked / `*` done marker, `ws/tab/label · status`, selection in reverse video, scrolls to keep it visible. Empty: `no other panes`; filtered empty: `no match`.
- Keys (picks what [How should the peek look and behave?](03-peek-ux.md) left open): typing filters (case-insensitive, name or status). While the filter is empty, `j`/`k` move and `q` closes. Esc clears a filter, then closes. Arrows and the wheel move, Enter or a row click peeks, `[x]` closes. In the mirror, `p`/Tab returns to the picker.
- **In place, no reopen needed in a float:** the fork has `float.set_hide_rule` + `float.resize`, so choosing a source sets the hide rule to it and resizes to ratio size. `p`/Tab clears the rule and goes back to 60%×50%. A modal popup switches in place at its current size until [Build: reopen helper, resize keys and remembered scale](13-build-resize.md).
- Source closed (`terminal.closed`/EOF) → fresh picker with a `source closed` notice (this replaces the keep-last-frame behavior from ticket 11). Each time the picker is shown, it reloads the snapshot.
- `src/main.rs`: peek is a `View::{Picker, Mirror}` switch. Mirror and picker share one row-diff `paint_body`, which writes nothing when no row changed.

**Next:** live smoke in the `mac_jr` VM (Ghostty, fork herdr), check each item, then write `## Resolution`, set `status: closed`, and add the Decisions-so-far line.

## Resolution

Verified live on 2026-10-18 in the `mac_jr` VM (fork herdr 0.9.5, Ghostty), with the progress build above:

- **Order:** blocked `!`, done `*`, working, idle, then plain shells in snapshot order. The focused pane is left out, and so is the peek's own pane when it runs in a tiled pane (`HERDR_PANE_ID`). Agent states came from `pane.report_agent` on test panes.
- **Keys:** a filter narrows by name or by status, case-insensitively. Backspace edits. Esc clears a filter and then, on an empty filter, closes. `j`/`k` move and stop at the ends. The wheel moves the selection. `zzz` shows `no match`. `q`, Esc and a `[x]` click each close the peek with exit 0.
- **Choosing:** Enter or a row click opens the mirror in place. The float gets the source as its hide rule and ratio size (`30x12`). `p` and Tab go back to a refreshed picker, and the float returns to `60%`×`50%` with no hide rule. A row whose pane closed after the picker was shown gives a `pane closed` notice and keeps the picker.
- **Source closed:** the peek returns to the picker with `source closed` and the float drops its rule.
- **How it was driven:** Ghostty in the VM stopped taking synthetic input after a macOS microphone prompt. Typing, clicks and AppleEvents were all ignored, even after a Ghostty restart. So the peek ran inside tiled pane `w1:p8` with `HERDR_FLOAT_ID` set to a real float, and was driven with `pane.send_text` (keys plus SGR mouse bytes) and `pane read`. The picker's float retargeting acted on that real float, which was checked through `float.list`.
- **Not verified live:** click-to-focus and key routing from Ghostty into a float picker. This is herdr's own routing, unchanged by this ticket, and it worked for the mirror in [Build: plugin skeleton, launcher and socket client](10-build-skeleton.md). Also not live: `no other panes` and scrolling past the window, which have unit tests only.
- **Found:** floats get `HERDR_FLOAT_ID` but no `HERDR_PANE_ID`. Float panes are not in `session.snapshot`, so the picker never lists itself anyway. Agent panes without a label all read `ws/tab/claude`, so the status is the only visible difference between them.
