---
title: Build: image mirror
type: task
status: closed
assignee: noomz
blocked_by: [09-spike-image-mirror.md, 11-build-text-mirror.md]
---

## Question

Capability check with text-mirror fallback; rasterize the `vt100` grid (bundled font, `PIP_FONT` override, box for missing glyphs) at target pixel size; kitty transmit double-buffered (alternate two ids, place new then delete old; one reused id flickers in Ghostty); own cell pixel size, `c`/`r` scale (popups report no pixel size); 10 fps cap, skip unchanged frames; letterbox; source resize → letterbox new ratio. Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md), [Does a kitty image render correctly inside a popup?](09-spike-image-mirror.md).

## Context added after [Build: reopen helper, resize keys and remembered scale](13-build-resize.md)

The global scale (`src/scale.rs`, `+`/`-`/`=`) already sizes the text mirror's float, so the image mirror reuses it rather than treating it as image-only. The popup fallback cannot resize: it saves the scale for the next peek.

## Resolution

Built in commit `accaf56` (`src/raster.rs`, `src/kitty.rs`, `Screen::paint_image` in `src/main.rs`).

- **Capability check:** `a=q` query followed by DA1 as the reply sentinel, 300 ms timeout, read straight from the fd before crossterm starts. Reply `i=31;OK` → image mirror; anything else → text mirror. The `CSI 16t` pixel check is dropped: [Does a kitty image render correctly inside a popup?](09-spike-image-mirror.md) showed popups never answer it.
- **Raster:** bundled JetBrains Mono Regular (`assets/`, OFL), `PIP_FONT=<path>` overrides; an unloadable `PIP_FONT` falls back to the bundled font and shows `PIP_FONT: <error>` in the top row. 14 px glyphs → 8×18 px cells. Whole source screen, one font cell per terminal cell; xterm 256 palette, bold brightens base colours, inverse, underline, wide cells, visible cursor as an inverted cell, box for missing glyphs. Fixed dark default colours (the peek cannot read the host theme).
- **Transmit:** `a=T,f=24,o=z`, 4096-byte base64 chunks, `C=1,q=2`; ids 7/8 alternate, new placed before old is deleted (`a=d,d=I`). Placement `c`/`r` = source shape letterboxed in cells below the top row, centred.
- **Rate:** at most every 100 ms, only when a frame arrived or the screen was invalidated (resize, scale, view switch). 80×24 renders in 0.18 ms + 0.27 ms zlib, ≈16 KB.
- **Lifecycle:** picker and close delete the image; source resize re-letterboxes on the 2 s size check; `+`/`-`/`=` reuse the global scale from [Build: reopen helper, resize keys and remembered scale](13-build-resize.md).

Verified live on 2026-10-19 in the `mac_jr` VM (Ghostty, fork herdr 0.9.5, real float from the plugin's `launch`, screenshots via `screencapture`):

| Check | Result |
|---|---|
| Image mirror in float, coloured, readable, letterboxed, clipped | pass |
| Live updates at 4 lines/s, no flicker across frames | pass |
| `+`/`-`/`=` re-fit the image to each float size | pass |
| `p` → picker, image gone; pick again → image back | pass |
| Source resized to 81 cols → new letterbox within 2.5 s | pass |
| Esc/`q` close, hide while source focused, Enter jump: no leftover image | pass |
| Probe unanswered (pty) → text mirror, no `a=T` sent | pass |

- CPU: ≈1.0% at 4 lines/s (text mirror 0.3%), ≈0.3% with the source idle (the 33 ms input tick, not image work).
- Not tested: `terminal.kitty_graphics = false` (needs a herdr restart; the unanswered-probe test covers the same path), a light host theme.
- Unit tests: raster cells, background, cursor, missing-glyph box, palette; chunking, id alternation, delete order, letterbox, probe reply parsing.
