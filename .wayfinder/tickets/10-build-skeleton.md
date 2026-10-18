---
title: Build: plugin skeleton, launcher and socket client
type: task
status: open
assignee: noomz
blocked_by: []
---

## Question

Cargo project, `herdr-plugin.toml` with the `plugin_action`, launcher that opens the popup (`PIP_SOURCE` → ratio size, else picker size), raw-socket JSON client (pane list, sizes, `pane.focus`), popup main loop with 2 std threads, top row with source/status and clickable `[x]`, Esc/`q` close, Enter or click on the mirror = jump to source (user, [Does the reopen helper resize the popup acceptably?](08-spike-reopen.md)). Links with `herdr plugin link`. Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md).

## Progress (2026-10-16, session paused; ticket stays claimed)

Built, not yet closed.

- Files: `Cargo.toml` (deps `crossterm` 0.29, `serde_json`), `herdr-plugin.toml`
  (action `peek` → `target/release/herdr-pip launch`; popup pane `peek` →
  `herdr-pip peek`), `.gitignore`, `src/herdr.rs` (socket client:
  `session.snapshot` → sources with `ws/tab/label`, agent status, size;
  `pane.focus`; `plugin.pane.open`), `src/main.rs` (`launch`: picker
  60%×50% or ratio `cols·s+2`×`rows·s+3`, s=50, clamp 30×8..90%×85% of the
  focused tab area; `peek`: raw mode, alt screen, `?1000h/?1006h`, dim top row
  + `[x]`, Esc/`q` close, Enter or body click = jump).
- Source size: rows = `scroll.viewport_rows`, cols = layout rect width minus
  (rect height − viewport_rows); may overshoot by a scrollbar gutter column.
- `cargo test` 4 pass; release build clean; `herdr plugin link .` done (plugin
  `herdr-pip` enabled).
- Live smoke: both launch paths open a popup (2nd open → `ui_busy`). User
  confirmed jump works (popup closes after jump, as decided). **Not yet
  confirmed:** popup ~64×34, top row text, `[x]` closes, Esc/`q` closes.
- Left to later tickets: observer thread + mirror (11), picker (12), scale
  persistence (13; suggest `HERDR_PLUGIN_STATE_DIR` over
  `$XDG_STATE_HOME/herdr-pip/scale`).
- Smoke surfaced minimize-while-source-focused + move/drag → out of scope,
  fork issue `~/Projects/Opensources/herdr/.local/prd/floating-popup.md`.

**Next:** get user confirmation on the four unconfirmed checks
(`HERDR_PLUGIN_ID=herdr-pip PIP_SOURCE=<pane> target/release/herdr-pip launch`),
then write `## Resolution`, set `status: closed`, add Decisions-so-far line.
