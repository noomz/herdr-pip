---
title: How should a small ratio popup fit its source?
type: grilling
status: closed
assignee: noomz
blocked_by: [06-popup-reopen.md, 07-graphics-protocol.md]
---

## Question

The user wants the peek small, in the source pane's aspect ratio, and resizable. This replaces the Size and Fitting answers of [How should the peek look and behave?](03-peek-ux.md). Decide:

- **Size formula:** default popup size from source cols×rows (e.g. a scale factor or % of screen area, with clamps), outer-border math (`--width/--height` include the border), where the top chrome row fits.
- **Fit:** the mirror is smaller than the source. Options: crop (bottom-left anchored, readable, partial), downscaled thumbnail (half-block/braille, whole pane, unreadable text), or a toggle between them.
- **Resize UX:** keys (e.g. `+`/`-`) mapped to reopen at a new size, if [Can a popup resize itself by reopening?](06-popup-reopen.md) says it works. Step sizes, limits, whether the size persists across peeks.
- **Picker in a small popup:** does the picker use the same small size, or open large and shrink once a source is chosen?

## Context

- Popups are always centered and modal; no positioning, no drag. Float + draggable is out of scope.
- Architecture is settled ([Which language and render architecture?](04-architecture.md)): observer at the source's full size, local `vt100` grid, raw ANSI row-diff paint. Crop or thumbnail are both paint-side choices on that grid.

## Context added after research

From [Can a popup resize itself by reopening?](06-popup-reopen.md):
- No in-place resize. A second open while a popup is up fails with `ui_busy`.
- Resize = close + reopen via a `setsid`-detached helper that retries `$HERDR_BIN_PATH plugin pane open ... --env PIP_SOURCE=<pane>` (closing kills the popup's whole session, so the popup can't reopen itself). Not live-tested.
- Expected cost: a frame with no popup during the swap, and a keystroke in that gap would reach the source pane's focus. Weigh this against simpler options: a fixed size per peek, or a size chosen in the Picker before opening.
- Clamps: outer at least 6×4 and at most the pane surface; percent 1–100.

## Resolution

Grilled with the user, 2026-10-16. Terms: [GLOSSARY.md](../../GLOSSARY.md) (new: image mirror, text mirror). Graphics facts: [Can a popup draw the mirror with a terminal graphics protocol?](07-graphics-protocol.md).

- **Fit:** **image mirror** by default: whole source pane rasterized by the plugin and sent with the kitty graphics protocol, scaled into the area below the top row, letterboxed. **Text mirror** (bottom rows, left columns) is the fallback when the capability check fails (no kitty `a=q` reply, no `CSI 16t` pixel size, `terminal.kitty_graphics = false`, tmux/ssh).
- **Size:** outer = `cols·s + 2` × `rows·s + 3` (border + own top row); default s = 50%; clamp min 30×8, max 90%×85%. Text mirror ignores s and uses 90%×85%; `+`/`-` do nothing there.
- **Resize:** `+`/`-` step s by 10 points in 20–100%, `=` resets to 50%; each change = close + reopen via a `setsid`-detached helper (`--env PIP_SOURCE=`). One global s remembered in `$XDG_STATE_HOME/herdr-pip/scale`. Accept the popup-less gap; live smoke test first, fallback = ratio only when opened with `PIP_SOURCE`, no `+`/`-`.
- **Picker:** fixed 60%×50%; choosing a source reopens at ratio size; `PIP_SOURCE` opens straight at ratio size; `p`/Tab reopens at picker size.
- **Font:** bundled open-licensed monospace (e.g. JetBrains Mono); `PIP_FONT=<path>` overrides; missing glyphs → box.
- **Frame rate:** image mirror capped at 10 fps, skip frames where the source hasn't changed (idle = nothing sent); text mirror keeps 30 fps.
- **Source resized mid-peek:** keep popup size, letterbox the new ratio; re-fit only on `=` or `+`/`-`.
