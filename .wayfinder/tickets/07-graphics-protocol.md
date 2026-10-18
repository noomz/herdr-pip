---
title: Can a popup draw the mirror with a terminal graphics protocol?
type: research
status: closed
assignee: research-agent
blocked_by: []
---

## Question

Could the mirror be a pixel image (kitty graphics, Sixel, iTerm2 inline) scaled to the popup, so its size no longer depends on cell grid / font size? Does herdr 0.9.5 pass graphics escapes from a plugin popup pane through to the host terminal (kitty, ghostty)? Can a pane learn cell pixel size? What would rasterizing terminal text to pixels cost (font rendering, CPU at 30 fps)?

Raised by user while grilling [How should a small ratio popup fit its source?](05-small-popup-fit.md); blocks it.

## Resolution

Findings: [research/07-graphics-protocol.md](../research/07-graphics-protocol.md). Not live-tested.

- **Kitty graphics works in a popup with no herdr change.** herdr's emulator (libghostty-vt) parses `ESC _G`, owns placement, clips popup images to the inner rect, deletes on close; file/temp/shm transmission accepted. Sixel and iTerm2 OSC 1337 are dropped; no DCS passthrough.
- **Pixel size is knowable:** `ws_xpixel/ws_ypixel` set, `CSI 14t/16t/18t` answered, but silent when the host cell size is unknown → needs a fallback.
- **Cost:** herdr has no "pane as image"; the plugin must rasterize the vt100 grid itself (font loading, glyph atlas, wide/emoji/nerd glyphs, bold/italic). Est. 2–6 ms blit + encode; ~6.5 MB RGBA per 200x50 frame → realistic 10–15 fps, not 30. kitty host gets inline base64 on the last hop.
- **Shape if adopted:** graphics mode behind a capability check (`a=q` / `16t` reply), text-cell paint as fallback (tmux, ssh, non-kitty, Windows).
