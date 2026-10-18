---
title: Does the reopen helper resize the popup acceptably?
type: task
status: closed
assignee: noomz
blocked_by: []
---

## Question

Live smoke test on herdr 0.9.5, before building resize. Throwaway script: open a popup, have a `setsid`-detached helper `popup.close` it and retry `plugin pane open --placement popup --width/--height --env PIP_SOURCE=` on `ui_busy`. Measure the popup-less gap, whether a keystroke in the gap reaches the underlying pane, and whether the helper survives the session kill. Outcome: keep reopen resize, or fall back to ratio-only-with-`PIP_SOURCE` and no `+`/`-`. Facts: [Can a popup resize itself by reopening?](06-popup-reopen.md).

## Resolution

**Keep reopen resize** (`+`/`-` stay). Live on herdr 0.9.5, macOS, 2026-10-16. Spike plugin kept in `research/08-spike-reopen/` (throwaway, unlinked).

- Recipe works 10/10 (alternating 40x10 ↔ 60x16): popup spawns a detached helper, waits for its ready marker, exits; helper retries `plugin pane open --placement popup --width --height --env …` on `ui_busy`. Always 2 attempts (1st `ui_busy` while old popup alive). New popup sees new PTY size and the `--env` state.
- Timing: old popup exit → helper's open accepted **12–18 ms**; exit → new process running 25–30 ms. Popup-less window ≈ one 16 ms render tick.
- **macOS has no `setsid` binary.** Detach with `POSIX::setsid()` (spike used `perl -MPOSIX -e 'setsid; exec @ARGV'`); in Rust, `pre_exec(|| libc::setsid())` on the spawned helper (or the binary re-execs itself in a helper mode). Popup must wait until the helper has called `setsid` before exiting, else the sweep can catch it.
- Control without setsid: helper got SIGHUP 1 ms after popup exit, no reopen. Detach is required, not optional.
- Flicker (user observed): brief flash of the underlying pane, acceptable for a resize key.
- Key leak in the gap: not a concern. User: PiP is view-only; acting on a pane means clicking to switch to it. → click on the mirror = jump to source (added to [Build: plugin skeleton, launcher and socket client](10-build-skeleton.md)).
- Not tested: `popup.close` path (same sweep, exit path suffices); a competing popup winning the singleton in the gap (helper would retry until its cap; spike capped at 400 attempts, build must cap too).
