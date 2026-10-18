---
title: How should the peek look and behave?
type: grilling
status: closed
assignee: noomz
blocked_by: [01-observe-stream.md, 02-popup-mechanics.md]
---

## Question

- Source selection: picker list, last-focused pane, pane by id, agents
  needing attention first?
- Size/placement: default popup size; how a source larger or smaller than the
  popup is fitted (render at popup size, crop which corner, follow cursor).
- Chrome: title line content (pane label, agent, status), `[x]` position.
- Keys inside the popup: Escape / `q` close; Enter jumps to the source pane?
  Anything else (switch source)?

## Context added after research

- Server crop is top-left: observing at popup size shows the source's *top*
  rows; prompts/status at the bottom are lost. Options: accept, observe at
  source size and pick bottom rows (needs a VT parser), or size popup to source.

## Resolution

Grilled with the user, 2026-10-16. Terms: [GLOSSARY.md](../../GLOSSARY.md).

- **Source selection:** the popup opens on a **picker** listing all panes except the focused one, attention-first then most-recently-active. Each row shows: attention marker, `workspace / tab / pane label`, then `· agent status` (nothing for plain shells). Type-to-filter, arrows/`j`/`k`, Enter peeks, click selects, wheel scrolls. The list is a snapshot, refreshed each time the picker is re-shown. `PIP_SOURCE=<pane id>` skips the picker.
- **Fitting:** the mirror is **bottom-anchored**: it shows the source's last rows and its left columns. This requires observing at source size and parsing locally (the architecture ticket picks how). The server's top-left crop is rejected because it hides agent prompts.
- **Size:** popup is `90%` × `85%` (outer).
- **Chrome:** herdr title is `PiP: <workspace>/<tab>/<label> · <status>`. The top inner row is self-drawn: dim source and status on the left, clickable `[x]` on the right. The mirror fills the rows below it.
- **Keys:** Esc or `q` closes. Enter **jumps** (focus the source pane, then close). `p` or Tab returns to the picker. All other input is ignored, so the peek is read-only. Mirror ignores wheel and other clicks.
- **Edge cases:** if the source closes (`terminal.closed`), return to the picker with a `source closed` notice. An empty picker shows `no other panes` and waits for Esc.
- **Keybind suggestion (README):** `prefix+shift+p` → `plugin_action`. It is free in herdr defaults and in the user's config. Fallback: `prefix+o`.
