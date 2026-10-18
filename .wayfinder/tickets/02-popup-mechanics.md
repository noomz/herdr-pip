---
title: How does a plugin popup behave?
type: research
status: closed
assignee: research-agent
blocked_by: []
---

## Question

For a plugin `[[panes]]` entrypoint opened with `placement = "popup"` on herdr 0.9.5:

- Mouse: are mouse events forwarded to the popup's program (SGR mouse
  reporting if the program enables it), so a drawn `[x]` can be clicked? Is
  there a herdr-drawn border/title, and does it have its own close affordance?
- Geometry: always centered? Exact inner size given `--width/--height`
  (border included?). Does it resize with the outer terminal (SIGWINCH)?
- Opening: from a `[[actions]]` command, can the plugin open its own popup
  (`plugin pane open --placement popup`) and pass the source pane id (env,
  `--env`, args)? What does `HERDR_PLUGIN_CONTEXT_JSON` contain for the
  underlying focused pane? Behavior when another popup/modal is open (`ui_busy`).
- Closing: command exit vs `popup.close`; focus restore afterwards.
- Can a keybinding open a plugin pane entrypoint directly, without an action?

## Resolution

Source-verified (full notes: [research/02-popup-mechanics.md](../research/02-popup-mechanics.md)).
- Mouse IS forwarded to the popup program (SGR, only if the program enables mouse reporting); herdr draws a plain border+title with NO close button, and clicks outside the inner rect are swallowed - so the `[x]` must be self-drawn and exit the process.
- Always centered; width/height are outer (border included); program sees outer-2 rows and outer-2(-1) cols; PTY resizes with the terminal (SIGWINCH).
- An action can run `herdr plugin pane open --placement popup --env K=V`; popup has no `HERDR_PANE_ID`, but `HERDR_PLUGIN_CONTEXT_JSON.focused_pane_id` is the underlying pane (computed from live state).
- Close = process exit or `popup.close` (socket only); focus is untouched. `ui_busy` in source = popup already open (docs claim more modals; not found in code).
- No keybinding type opens a plugin pane directly; use `plugin_action` (or `shell` calling the CLI).
