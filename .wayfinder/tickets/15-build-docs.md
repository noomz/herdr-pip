---
title: Build: README, keybind and install docs
type: task
status: closed
assignee: noomz
blocked_by: [11-build-text-mirror.md, 12-build-picker.md, 13-build-resize.md, 14-build-image-mirror.md]
---

## Question

README: install via `herdr plugin link`, keybind `prefix+shift+p` (fallback `prefix+o`), keys, `PIP_SOURCE`/`PIP_FONT`, image vs text mirror and when each is used. Final smoke test on herdr 0.9.5.

## Context added after [Build: image mirror](14-build-image-mirror.md)

Document `PIP_FONT`, the kitty-graphics requirement for the image mirror (text mirror otherwise), and the fixed dark palette (a light host theme shows a dark picture). Bundled font licence: `assets/JetBrainsMono-OFL.txt`.

## Resolution

Done on 2026-10-19.

- `README.md`: what the peek is (float on herdr with floats, modal popup on stock 0.9.5), install (`cargo build --release` then `herdr plugin link .`; link runs no build), keybind snippet (`prefix+shift+p` → `plugin_action` `herdr-pip.peek`, fallback `prefix+o`), `herdr plugin action invoke herdr-pip.peek`, picker and mirror key tables, jump behaviour per float/popup, size and scale file, image vs text mirror (kitty graphics probe), fixed dark palette, `PIP_FONT` (inherited from the herdr server's environment; herdr only strips specific keys from pane env), `PIP_SOURCE`, bundled font licence.
- `herdr-plugin.toml` description now says "float or popup".

Final smoke on **stock herdr 0.9.5** (`~/.local/bin/herdr.bak-0.9.5` in the `mac_jr` VM, no `float.*` methods), run in an isolated `XDG_CONFIG_HOME=~/stockcfg` with the README keybind pasted verbatim and the plugin linked; Ghostty, keys sent as CGEvents:

| Check | Result |
|---|---|
| `herdr plugin link`, `plugin action list` shows `peek` | pass |
| README keybind `ctrl+b` `P` opens the modal popup on the picker (60%×50%), focused pane excluded | pass |
| Enter → image mirror of a 4 lines/s ticker, live, letterboxed | pass |
| `+` → scale file 60, top row `~/1/ticker · 60% · resize needs floats`, popup keeps its size | pass |
| `=` → scale 50 | pass |
| Enter → jump: popup closes, ticker pane focused, no peek process | pass |
| reopen via keybind, Esc → closes, no peek process | pass |

- Found: a popup opened from the picker stays at picker size for the mirror (no reopen helper, per [Build: reopen helper, resize keys and remembered scale](13-build-resize.md)); README says so.
- Not re-tested on stock: `[x]` click and `q` (same code path as the fork, verified in [Build: plugin skeleton, launcher and socket client](10-build-skeleton.md)), text mirror (verified via the unanswered-probe test in [Build: image mirror](14-build-image-mirror.md)).
- VM restored afterwards: stock server stopped, Ghostty back on the fork herdr; `~/stockcfg` and `~/stock.sh` left for reuse.
