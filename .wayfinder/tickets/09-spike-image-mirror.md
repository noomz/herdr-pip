---
title: Does a kitty image render correctly inside a popup?
type: task
status: closed
assignee: noomz
blocked_by: []
---

## Question

Live smoke test in kitty and ghostty under herdr 0.9.5. Throwaway program in a popup: run the capability check (`a=q` reply, `CSI 16t` pixel size), transmit one RGBA image with a reused id, re-transmit at 10 fps, resize/close the popup. Check: clipped to inner rect, no leftover on close, no flicker on re-transmit, behaviour when `16t` is silent, kitty (inline base64) vs ghostty bandwidth. Facts: [Can a popup draw the mirror with a terminal graphics protocol?](07-graphics-protocol.md).

## Resolution

**Image mirror works in both, with a double-buffer.** Live on herdr 0.9.5, 2026-10-16: kitty on the host (macOS 27), Ghostty 1.3.1 in the `aero-probe` tart VM. Spike plugin kept in `research/09-spike-image-mirror/` (throwaway, unlinked; `spike-ghostty.log` is the VM run).

- Capability: `a=q` → `i=31;OK` in both (herdr answers as kitty-capable). DA1 (`CSI c`) works as reply sentinel; probe round-trip <1 ms.
- **Pixel size is unknown inside a popup in both terminals:** `CSI 16t` silent, `TIOCGWINSZ` pixels 0x0. Build must not depend on it: rasterize at its own cell size (font metrics, e.g. 9x18) and let explicit `c`/`r` scale to the popup. Scaling looked right in both.
- Clip: image + red edge border fully inside the popup inner rect in both; no overflow.
- Close: no leftover image in kitty after exit without explicit delete (herdr cleans up). Ghostty: same visual result.
- **Flicker:** kitty none with one reused id. Ghostty flickers with one reused id; **gone when alternating ids 7/8 and deleting the previous one after placing the new one** (`PIP_ALT=1`). Build uses the double-buffer always.
- Cost (Python spike, 873x486 image, zlib level 1, `o=z`): ~55–85 KB compressed, ~75–110 KB on the wire per frame, encode ~2 ms, write ~1.5 ms. 10 fps steady. Ghostty vs kitty bandwidth: no visible difference at this size.
- Not tested: popup resize while streaming (no SIGWINCH test), source images inside the mirrored pane.

**Destination redrawn (user, same session):** the modal popup blocks all work while open, so a popup-only PiP is not useful. New destination: popup peek **plus pin to a non-modal split pane mirror** (`plugin pane open --placement split --direction --no-focus`, a normal herdr pane, input stays with the user's pane). True floating PiP needs a herdr core fork: wanted later as a separate effort, still out of this map. Open question → [How should a pinned split mirror behave?](16-pin-split.md).
