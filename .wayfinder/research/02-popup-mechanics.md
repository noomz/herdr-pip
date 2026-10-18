# 02 - Plugin popup mechanics (herdr 0.9.5)

Method: static source reading of `/Users/noomz/Projects/Opensources/herdr`
(checkout is `preview-2026-10-12-...-g78bd946e`, i.e. slightly past 0.9.5; popup
code is the same shape as documented). Paths are relative to that repo. No popup
was opened. Docs: `docs/next/website/src/content/docs/` (D = plugins.mdx,
C = configuration.mdx, CLI = cli-reference.mdx).

## Mouse

- **Mouse is forwarded to the popup program.** Client side: a left/any Down inside
  the popup's `inner_rect` is pushed as a `ClientInputTarget::Popup` mouse event
  (`src/client/shell/mouse.rs:923-948`); Moved only if `mouse_reporting`
  (`:940`); wheel always (`:942-947`); Up/Drag are forwarded via a gesture that
  starts on Down when `hit.mouse_reporting` (`:927-936`, `:882-915`). Coordinates
  are relative to the inner rect (`:2618-2622`). SGR-pixel is used only if the
  program enabled it (`:2623`).
- **Server encodes only if the program enabled mouse reporting.** Popup input
  goes through `apply_client_popup_input_events` -> `apply_client_terminal_input_events`
  (`src/server/pane_input.rs:220-228`, `:240-300`) which calls
  `encode_mouse_button`; that returns `None` (nothing sent) unless
  `mouse_reporting_enabled()` (`src/pane.rs:3748-3757`). `mouse_reporting` and
  `sgr_pixel_mouse` are read from the popup runtime each frame
  (`src/server/client_shell.rs:536-537`). So the program must send `?1000h`
  (or 1002/1003) plus `?1006h`; a click is then delivered as an SGR report
  (`ESC[<0;col;rowM` / `m`), 1-based relative to the program's own screen.
- **Wheel**: routed by `wheel_routing()` - MouseReport if enabled, else
  AlternateScroll, else herdr scrollback (`pane_input.rs:139-164`).
- **herdr draws a border + title, no close affordance.** `composition.rs:583-590`:
  `Block` with `Borders::ALL`, accent border, `.title(popup.title)`. Title is the
  manifest pane `title` (`src/app/api/plugins/panes.rs:38-40`; `"popup"` fallback,
  `client_shell.rs:515-519`). There is no `[x]`, button, or hit target on the
  border. A click anywhere outside `inner_rect` (border, outside popup) is
  swallowed: `mouse.rs:923-950` returns without action while a popup hit exists.
  Esc is not a close key either: popup gets all keys (`src/client/shell/input.rs:643-645`, D line ~310).
  => the drawn `[x]` must live inside the program's own canvas and be handled
  from SGR mouse events; the program must `exit` (or call `popup.close`).

## Geometry

- **Always centered**: `outer_x = area.x + (area.width - outer_width)/2`, same for y
  (`src/popup_size.rs:71-72`). Area = `view.terminal_area` (the pane surface, i.e.
  excluding sidebar/tab bar, `composition.rs:576` uses `layout.pane_surface`;
  `src/app/popup.rs:140-147`).
- **`--width/--height` are OUTER dimensions, border included**
  (`popup_size.rs:57-66`; D "outer terminal-cell dimensions"; C "Dimensions include
  the popup border"). Cells or `N%` of the area; default half of area, min outer
  6x4, clamped to area (`popup_size.rs:55-69`). Too small an area -> open fails
  "terminal area too small for popup" (`popup.rs:148-152`).
- **Program-visible size = outer - 2 (border) in rows; cols = outer - 2, minus 1
  more when that is > 4** (`popup_size.rs:73-85`; one column reserved inside
  the border). Test confirms 100x30 area, 80%/40% -> outer 80x12, inner 77x10
  (`src/app/api/plugins/mod.rs:2447-2451`). The PTY is created at inner rows/cols
  (`popup.rs:153-155`).
- **Resizes with the outer terminal**: geometry is recomputed every render and
  the runtime resized (`ui/panes.rs:530-552`, `server/client_shell.rs:541-562`
  calling `runtime.resize(rows, cols, ...)`), which resizes the PTY (SIGWINCH).
  Percent sizes track the terminal; cell sizes stay fixed but are clamped.
  Skipped only if `direct_attach_resize_locks` holds the terminal.
- Singleton: one popup per session; per-client ownership via `popup_owner_tab_id`
  (`src/server/headless.rs:2015`, `:2204-2206`).

## Opening

- **An action command can open its own popup**: `herdr plugin pane open --plugin ID
  --entrypoint ID --placement popup [--width ..] [--height ..] [--env K=V]`
  (CLI line 521; `--width/--height` are hidden flags in clap help but accepted,
  `src/cli/plugin/bindings.rs:122-135`, `:441-455`). It is just a socket call
  (`plugin.pane.open`), so it works from any process with `HERDR_SOCKET_PATH`/
  `HERDR_BIN_PATH` (D lines ~286-296). Placement can also be set in the manifest
  `[[panes]]` (D 322-330).
- **Constraints**: for popup (and overlay) `--workspace`, `--target-pane`,
  `--direction` are rejected with `invalid_params`; popup always targets the
  active pane (`app/api/plugins/mod.rs:505-518`). `--width/--height` rejected for
  non-popup placement (`:492-500`).
- **Passing the source pane id**: (1) `--env KEY=VALUE` is passed through, except
  protected keys (`HERDR_SOCKET_PATH, HERDR_ENV, HERDR_PLUGIN_ID/ROOT/CONFIG_DIR/STATE_DIR/ENTRYPOINT_ID/CONTEXT_JSON, HERDR_BIN_PATH`)
  which are stripped (`panes.rs:246-280`, `:356-369`). A custom var such as
  `PEEK_SOURCE_PANE=$HERDR_PANE_ID` therefore works (the action process gets
  `HERDR_PANE_ID` = focused pane, D ~270). (2) The popup's own
  `HERDR_PLUGIN_CONTEXT_JSON` is built from *live* state at open time, not copied
  from the action (`panes.rs:18`, `app/api/plugins/context.rs:32-37`).
- **Context JSON contents** (`src/api/schema/plugins.rs:382-413`, built in
  `context.rs:347-382`): `workspace_id, workspace_label, workspace_cwd, worktree,
  tab_id, tab_label, focused_pane_id, focused_pane_cwd, focused_pane_agent,
  focused_pane_status, selected_text (always null here), invocation_source
  ("api"), correlation_id ("plugin-pane"), clicked_url, link_handler_id`.
  Because the popup has no pane id and does not take focus, `focused_pane_id`
  is the underlying tiled pane (test `mod.rs:2467`, D ~337-338).
- **Popup env**: `HERDR_PANE_ID` is removed (`src/pane.rs:178-181`, `:211-215`;
  test asserts `unset`, `mod.rs:2403-2441`), so the source pane must come from
  CONTEXT_JSON or `--env`. (Keybinding popups additionally get
  `HERDR_ACTIVE_PANE_ID/_TAB_ID/_WORKSPACE_ID/_PANE_CWD`, `custom_commands.rs:249-285`; plugin popups do not.)
- **`ui_busy`**: in source the only `ui_busy` is "a popup pane is already open"
  (`mod.rs:501-502`; test `:2430-2435`). Docs additionally claim Settings/Copy
  mode/other modals return `ui_busy` (D ~340); I found no such check in
  `spawn_popup_command` (`popup.rs:116-132` checks only existing popup, active
  workspace, focused pane) - the test even opens in `Mode::Navigate`
  (`mod.rs:2429`). Opening a popup while Settings overlay is up closes the
  settings overlay client-side (`client/shell/state.rs:1747`). Other errors:
  `no active workspace`, `plugin_pane_open_failed`.
  Implication: a second `plugin pane open` (e.g. toggling from the same key while
  PiP is open) returns `ui_busy` - but the keys never reach herdr while the popup
  is open (see below), so toggle must be done by the popup itself.

## Closing

- **Command exit**: `PaneDied` for the popup pane id -> `close_popup_pane()`
  (`app/api.rs:210-218`). **`popup.close`** socket request does the same, errors
  `popup_not_open` otherwise (`api.rs:1212-1217`; method list
  `api/schema/operations.rs:202`; D 311; socket-api.mdx 621). There is no
  `herdr popup close` CLI command in cli-reference.mdx (only `plugin pane close`
  for real panes); use the socket (JSON line to `HERDR_SOCKET_PATH`) or simply
  exit the process.
- **Cleanup/focus restore**: `close_popup_pane` removes terminal + runtime and
  sets `mode = Terminal` (or Navigate with no workspace) (`popup.rs:16-33`).
  Tiled focus was never changed (popup is not a pane; `app/state.rs:1290-1300`
  invariants; D ~335-338), so focus "restore" is implicit: same pane, same zoom.

## Keybinding

- `[[keys.command]]` types are `shell | pane | popup | plugin_action`
  (`src/config/keybinds.rs:112-118`, `:149-157`). **No type opens a plugin pane
  entrypoint directly**; `plugin_action` takes an action id (C ~226-235,
  `custom_commands.rs:215-225`). Options for the user:
  1. `type="plugin_action"` -> action command runs `herdr plugin pane open ... --placement popup` (the intended path).
  2. `type="shell"` with `command = "herdr plugin pane open --plugin X --entrypoint Y --placement popup"` (shell commands get `HERDR_SOCKET_PATH`/`HERDR_BIN_PATH` and `HERDR_ACTIVE_*`, `custom_commands.rs:249-285`).
  3. `type="popup"` running the program directly: works and gets `HERDR_ACTIVE_PANE_ID`,
     but it bypasses the plugin (no `HERDR_PLUGIN_*`, title is "popup", config lives in user's config.toml rather than the plugin).
- While a popup is open, all keyboard input (including the prefix and Esc) goes to
  the popup (`client/shell/input.rs:643-645`), so keybinds cannot toggle it.

## Implications for the peek popup

- Mouse clicks on a self-drawn `[x]` are viable: enable `?1000h ?1006h` (plus `?1003h` only if hover is wanted) in the popup program, parse SGR reports, hit-test against cell coords relative to the program's own screen (0-based inner cell + 1). Nothing outside the inner rect (herdr's border/title) is clickable and herdr offers no close button - the `[x]` must exit the process (or call `popup.close`).
- Size the UI from the PTY winsize at start and on SIGWINCH; usable area is outer minus border minus one reserved column (e.g. `--width 60 --height 20` -> 57x18 inner). Use percent sizes if it should follow the terminal; it is always centered, never positionable.
- Open path: `[[actions]]` command -> `"$HERDR_BIN_PATH" plugin pane open --plugin "$HERDR_PLUGIN_ID" --entrypoint peek --placement popup --env PEEK_SOURCE_PANE="$HERDR_PANE_ID"`; the popup can alternatively read `focused_pane_id` from `HERDR_PLUGIN_CONTEXT_JSON`. `HERDR_PANE_ID` is NOT set inside the popup, so do not rely on it.
- Pass the target explicitly via `--env` if the mirrored pane may differ from the focused one (context is recomputed from live focus at open time, ignoring the action's own context); protected `HERDR_*` plugin vars cannot be overridden.
- Docs' `ui_busy` for Settings/Copy mode is not backed by source in this checkout; only "popup already open" is. Handle `ui_busy`/non-ok exit codes in the action regardless and surface a message.
- Keybinding cannot target a pane entrypoint directly; ship a `[[actions]]` and tell users to bind it with `type = "plugin_action"` (or `shell` running the CLI). Close-on-exit gives clean focus restore for free.
