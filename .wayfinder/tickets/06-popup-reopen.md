---
title: Can a popup resize itself by reopening?
type: research
status: closed
assignee: research-agent
blocked_by: []
---

## Question

On herdr 0.9.5, can a running popup plugin process change its own popup size?

- Can a process inside a popup run `herdr plugin pane open --placement popup --width/--height ...` (or socket equivalent) while its popup is open? Popup is a singleton: does the new open replace the old one, error, or queue? Which env/ids does the child need (`HERDR_*` is stripped)?
- If replace works: does the old process get killed (SIGHUP?) or must it exit first? Is there a visible flicker or a frame with no popup?
- Hand-off of state: can the new process get the source pane id (`--env PIP_SOURCE=`) and the new size, so it skips the picker?
- Any in-place resize path (socket method, hidden flag) that avoids reopening?
- Min/max clamps on `--width/--height` cells.

## Resolution

No. Popup is a singleton: reopening while open returns `ui_busy` (no replace, no queue), and there is no in-place resize method (only `popup.close`). Self-resize = close then reopen; closing SIGHUPs/TERMs/KILLs the popup's whole session synchronously, so the reopen must come from a detached (`setsid`) helper that retries on `ui_busy`. Child still has `HERDR_BIN_PATH`, `HERDR_SOCKET_PATH`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ENTRYPOINT_ID`, `HERDR_PLUGIN_CONTEXT_JSON` (not `HERDR_PANE_ID`); hand state via `--env PIP_SOURCE=`. Clamp: outer min 6x4, max pane surface, percent 1..=100. A popup-less frame during the swap is likely (unmeasured). Details: [research/06-popup-reopen.md](../research/06-popup-reopen.md).
