---
title: Picture-in-picture pane peek for herdr
label: wayfinder:map
---

## Destination

A working, linkable herdr plugin (`herdr-pip`) that opens a **modal popup**
over the current view showing a **live mirror of one chosen pane**, closable
with a clickable `[x]` button (and Escape), and can **pin** that mirror into a
**non-modal split pane** so the user keeps working while it updates. Installed
with `herdr plugin link` and smoke-tested on herdr 0.9.5.
(Redrawn 2026-10-16: a modal-only popup blocks all work, see
[Does a kitty image render correctly inside a popup?](tickets/09-spike-image-mirror.md).)

## Notes

- Domain: herdr plugin API = herdr CLI + socket API + `herdr-plugin.toml`.
  Docs: `~/Projects/Opensources/herdr/docs/next/website/src/content/docs/`
  (`plugins.mdx` Panes section, `cli-reference.mdx` terminal session observe,
  `socket-api.mdx`); source: `~/Projects/Opensources/herdr/src/`.
- Known constraints (from charting): popup is session-modal (gets all input
  incl. Escape), a singleton, opened via `plugin pane open --placement popup
  [--width --height]`, closes when its command exits or on `popup.close`.
  `terminal session observe <target>` streams read-only `terminal.frame`
  records (base64 ANSI), many observers allowed.
- Prior art: runtime-cost research in
  `~/Projects/Opensources/herdr-i3/.wayfinder/research/03-runtime-cost.md`
  (std-only Rust on raw socket ≈2.5 ms; `link` skips `[[build]]`; command
  `target/release/<bin>` resolves against plugin dir).
- **Execution override:** user wants research → plan → implement; the map ends
  with build tickets that produce the plugin.
- Standing preference: fast, least CPU (a live mirror redraws continuously).
- No upstream contribution: never open herdr issues or PRs from this effort.
- Skills for grilling tickets: `grilling` + `domain-modeling`.

## Decisions so far

<!-- one line per closed ticket -->

- [What does the terminal observe stream deliver?](tickets/01-observe-stream.md): CLI-only (binary client protocol, no JSON socket method); NDJSON server-rendered ANSI frames (first full `ESC[2J`, then cell diffs, sync-wrapped, ≤60 fps, idle ≈ silent); `--cols/--rows` never resizes source but **crops top-left** (bottom rows lost); current viewport only, no scrollback; pass-through needs no VT parser.
- [How does a plugin popup behave?](tickets/02-popup-mechanics.md): mouse forwarded inside inner rect if program enables `?1000h`/`?1006h` → drawn `[x]` works; herdr border+title, no close button; centered, outer size incl. border (80x12 → 77x10 inner), SIGWINCH on resize; open via action → `plugin pane open --placement popup --env PIP_SOURCE=…` (custom env; `HERDR_*` stripped); close = exit or socket `popup.close`; popup eats all keys incl. prefix.
- [How should the peek look and behave?](tickets/03-peek-ux.md): picker-first (attention-sorted, excludes focused, snapshot, `PIP_SOURCE` skips); ~~bottom-anchored mirror; popup 90%×85%~~ (size + fit superseded, see [How should a small ratio popup fit its source?](tickets/05-small-popup-fit.md)); own top row with source+status and clickable `[x]`; Esc/`q` close, Enter jump, `p`/Tab picker, else read-only; source-closed → picker; keybind `prefix+shift+p`.
- [Which language and render architecture?](tickets/04-architecture.md): Rust; spawn `terminal session observe` at source's full size → `vt100` grid; raw-socket JSON for list/size/`pane.focus` jump; raw ANSI row-diff paint, 30 fps cap; `crossterm` input only; 2 std threads; restart observer on SIGWINCH. Deps: `vt100`, `crossterm`, `serde_json`, `base64`. (Image mirror adds a font rasterizer and 10 fps cap, see [How should a small ratio popup fit its source?](tickets/05-small-popup-fit.md).)
- [Can a popup resize itself by reopening?](tickets/06-popup-reopen.md): No in-place resize (only `popup.close`); reopen while open = `ui_busy` (no replace/queue); close SIGHUP→TERM→KILLs the popup's whole session synchronously, so reopen needs a detached `setsid` helper retrying on `ui_busy`; child keeps `HERDR_BIN_PATH`/`HERDR_SOCKET_PATH`/`HERDR_PLUGIN_ID`/`HERDR_PLUGIN_ENTRYPOINT_ID`, hand state via `--env PIP_SOURCE=`; clamp outer min 6x4, max pane surface; popup-less frame likely (unmeasured).
- [Does the reopen helper resize the popup acceptably?](tickets/08-spike-reopen.md): yes, keep `+`/`-`; exit→reopen 12–18 ms, brief flash judged fine; macOS lacks `setsid` binary → `setsid()` syscall in helper, popup waits for it; without detach helper dies; key leak moot (PiP view-only, click mirror = jump).
- [Can a popup draw the mirror with a terminal graphics protocol?](tickets/07-graphics-protocol.md): yes via kitty graphics only (herdr re-emits + clips popup images, no herdr change; Sixel/iTerm2 dropped); pixel size via `16t`/winsize (may be absent); plugin must rasterize text itself, est. 10–15 fps, text-cell fallback required. Untested live.
- [How should a small ratio popup fit its source?](tickets/05-small-popup-fit.md): image mirror (kitty, self-rasterized, bundled font + `PIP_FONT`, 10 fps, skip unchanged) with text-mirror fallback at 90%×85%; outer `cols·s+2`×`rows·s+3`, s=50% (20–100%, `+`/`-`/`=`, remembered globally) via detached reopen helper; picker 60%×50%; source resize → letterbox.
- [Does a kitty image render correctly inside a popup?](tickets/09-spike-image-mirror.md): yes in kitty and Ghostty 1.3.1; popups report no pixel size (`16t` silent, winsize 0) → own cell px + `c`/`r` scale; reused id flickers in Ghostty → double-buffer two ids; ~75–110 KB/frame wire, clean clip and close. Destination redrawn: popup peek + pin to non-modal split.
- [Build: plugin skeleton, launcher and socket client](tickets/10-build-skeleton.md): the peek opens as a fork-herdr float, hidden while its source is focused; Jump keeps the float running; modal popup is the fallback on herdr without floats.
- [Build: text mirror](tickets/11-build-text-mirror.md): observe at source size → `vt100` (one spare column) → row-diff paint; anchored to content end, not the last row; float peeks re-read source size every 2 s and restart the observer on change.
- [Build: picker](tickets/12-build-picker.md): attention-first list (blocked, done, working, idle, shell; then newest state change), focused and own pane excluded; type-to-filter, `j`/`k`/`q` only on an empty filter, Esc clears then closes; a float switches picker ↔ mirror in place via `float.set_hide_rule` + `float.resize` (no reopen); source closed → picker with notice.
- [Build: reopen helper, resize keys and remembered scale](tickets/13-build-resize.md): float-only, no reopen helper; `+`/`-` step a global scale by 10 in 20–100, `=` resets to 50 and refits; float resizes in place; popup fallback saves the scale for the next peek and says `resize needs floats`; scale in `$XDG_STATE_HOME/herdr-pip/scale`, applied to the text mirror too.

## Not yet specified

- **Testing approach.** Pure render/crop logic unit tests vs live smoke script.
  Ghostty smoke can run in the `aero-probe` tart VM (Ghostty 1.3.1, herdr,
  standalone Python at `~/python` installed there).
- **Build: pin to split.** Build ticket for the pinned mirror; graduates once
  [How should a pinned split mirror behave?](tickets/16-pin-split.md) closes.

## Out of scope

- Non-modal floating / draggable PiP (typing underneath while it floats; popups are always centered and modal): needs a herdr
  core floating layer; not possible in plugin v1. User wants it (2026-10-16)
  as the follow-up **fork-core effort** after this map; the pinned split is the
  plugin-only stand-in. Also there: minimize the peek while its source pane is
  focused and restore it on leaving (user, 2026-10-16 smoke test; close kills
  the popup session, no hide exists). Written up as a fork issue:
  `~/Projects/Opensources/herdr/.local/prd/floating-popup.md`.
- Previewing a whole tab as a composite of its panes.
- Marketplace publishing.
- Sixel / iTerm2 image mirrors (herdr drops them).
