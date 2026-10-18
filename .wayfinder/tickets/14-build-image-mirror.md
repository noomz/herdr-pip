---
title: Build: image mirror
type: task
status: open
assignee:
blocked_by: [09-spike-image-mirror.md, 11-build-text-mirror.md]
---

## Question

Capability check with text-mirror fallback; rasterize the `vt100` grid (bundled font, `PIP_FONT` override, box for missing glyphs) at target pixel size; kitty transmit double-buffered (alternate two ids, place new then delete old; one reused id flickers in Ghostty); own cell pixel size, `c`/`r` scale (popups report no pixel size); 10 fps cap, skip unchanged frames; letterbox; source resize → letterbox new ratio. Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md), [Does a kitty image render correctly inside a popup?](09-spike-image-mirror.md).

## Context added after [Build: reopen helper, resize keys and remembered scale](13-build-resize.md)

The global scale (`src/scale.rs`, `+`/`-`/`=`) already sizes the text mirror's float, so the image mirror reuses it rather than treating it as image-only. The popup fallback cannot resize: it saves the scale for the next peek.
