---
title: Which language and render architecture?
type: grilling
status: closed
assignee: noomz
blocked_by: [01-observe-stream.md]
---

## Question

- Language: Rust binary (per herdr-i3 runtime research) vs other.
- Transport: raw socket stream vs spawning `herdr terminal session observe`.
- Render path: pass-through server-rendered frames vs own VT parse +
  re-render; dependency choice (e.g. `vt100`, `crossterm`) and why.
- Redraw throttling to meet the least-CPU preference.

## Context added after research

- Observe is CLI-only (binary protocol), so transport = spawn
  `herdr terminal session observe`, unless the binary protocol is reimplemented.
- Pass-through frames use absolute positioning + `ESC[2J`, so reserving a row
  for a self-drawn `[x]`/title needs either a VT parser (composite) or a
  terminal trick (scroll region + origin mode) — weigh which is robust.

## Context added after peek UX

- UX chose a **bottom-anchored mirror** with a self-drawn top row, so the source
  must be observed at its full size and parsed locally. Pass-through of server
  crops is ruled out. The question is now which VT parser and render path is
  cheapest. See [How should the peek look and behave?](03-peek-ux.md).
- Mirror height is popup inner rows − 1. Jump needs a focus call on the
  source pane (CLI or socket).

## Resolution

Grilled with the user, 2026-10-16.

- **Language:** Rust, single binary `target/release/herdr-pip`; README tells the user to `cargo build --release` (`link` skips `[[build]]`).
- **Mirror transport:** spawn `herdr terminal session observe <id> --cols W --rows H` as a child, W×H = the source's full viewport size; read NDJSON, base64-decode `bytes`. No reimplementation of the binary client protocol. Size change = restart the child.
- **Control transport:** raw JSON over the herdr socket (`UnixStream`, one short request per call) for picker list, source size, jump, popup close. No CLI spawns per call, no `events.subscribe`.
- **Jump:** socket `pane.focus {pane}` (`src/api/schema.rs:116`; in the schema, not in `socket-api.mdx`). `agent focus` is rejected: it resolves agent panes only.
- **VT parser:** `vt100` crate. Rejected: hand-rolled parser (must own wide chars + clipping, brittle to server grammar), herdr's `ghostty-vt` (Zig toolchain), `alacritty_terminal` (heavier).
- **Output:** raw ANSI, no TUI framework. Per paint, compare each visible row's formatted bytes to last drawn, re-emit changed rows only, batch wrapped in `?2026h/l`. Top row redrawn only when its text changes.
- **Input/mode:** `crossterm` for raw mode, alt screen, mouse enable (`?1000h`/`?1006h`), event parsing. Output stays raw.
- **Resize / source gone:** query source size at peek start and on own SIGWINCH, then restart observer. Popup is modal, so panes can't be rearranged mid-peek. Accepted edge: automation resizing the source mid-peek shows a stale crop until the next SIGWINCH. Source gone = `terminal.closed` or EOF → picker.
- **Throttle:** parse every frame (keeps state correct, drains stdout before the 30 s stall drop), paint at most 30 fps and only when visible rows changed.
- **Concurrency:** two std threads (observer reader → `Mutex<vt100::Parser>` + dirty flag; main = input, 33 ms paint tick, socket calls). No async runtime.
- **JSON:** `serde_json` + `base64`. Final deps: `vt100`, `crossterm`, `serde_json`, `base64`.

**UX change surfaced here:** the user replaced the 90%×85% bottom-anchored popup with a *small popup in the source's aspect ratio, resizable* (asked for float + draggable as well; ruled out: popups are always centered and modal, see [How does a plugin popup behave?](02-popup-mechanics.md)). The decisions above hold for either fit (crop or thumbnail). Fit and size reopen in [How should a small ratio popup fit its source?](05-small-popup-fit.md).
