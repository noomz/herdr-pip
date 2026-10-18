# 06 - Can a popup resize itself by reopening? (herdr 0.9.5)

Method: static source reading of `/Users/noomz/Projects/Opensources/herdr` (HEAD `78bd946e`, slightly past 0.9.5; `herdr --version` -> `herdr 0.9.5`). Paths are relative to that repo; docs under `docs/next/website/src/content/docs/` (PL = plugins.mdx, CLI = cli-reference.mdx, SOCK = socket-api.mdx). Builds on `research/02-popup-mechanics.md` (singleton, geometry, env basics); those facts are not repeated. No live probe was run: a test popup needs a plugin entrypoint, and none of the six installed plugins is ours. Timing claims below (flicker) are therefore reasoned from code, not measured.

## 1. Reopen while open: error, not replace or queue

- A second `plugin.pane.open` with `placement=popup` while a popup exists returns error `ui_busy` ("a popup pane is already open"): `src/app/api/plugins/mod.rs:501-503`. The check runs before any spawn. A second guard sits in `spawn_popup_command`: `Err("popup already open")` (`src/app/popup.rs:116-118`), surfaced as `plugin_pane_open_failed` (`panes.rs:28-30`).
- There is no replace path and no queue: the only writers of `state.popup_pane` are the spawn (`popup.rs:162,186`) and `close_popup_pane` (`popup.rs:14-16`, `.take()`). Test pins the error: `mod.rs:2434-2437`.
- So "reopen with a new size" is necessarily close-then-open, two separate socket requests.

## 2. Old process: SIGHUP, synchronous, whole session

- Popup ends via (a) child exit -> `PaneDied` -> `close_popup_pane()` (`src/app/api.rs:210-218`) or (b) `popup.close` socket request -> same function (`api.rs:1212-1217`; schema `src/api/schema.rs:131`).
- `close_popup_pane` calls `shutdown_terminal_runtime` (`popup.rs:20-22`, `src/app/runtime.rs:30-34`) -> `PaneRuntime::shutdown` (`src/pane.rs:2234-2246`) -> `shutdown_pane_processes` (`pane.rs:1760-1805`): collects every pid in the child's **session** (`session_processes`: `getsid` match on macOS `src/platform/macos.rs:1126-1139`, `/proc` session id on Linux `src/platform/linux.rs:800-823`), then SIGHUP, wait up to 250 ms, SIGTERM, 250 ms, SIGKILL, 250 ms. The same function runs when the child had already exited (cleanup of stragglers). This runs inline on the app thread, so the close request does not return until the session is dead (up to ~750 ms worst case, normally as fast as the process dies).
- Consequence for the popup process: if it calls `popup.close` (or `plugin pane open` after) it will be SIGHUP'd mid-flight; anything it spawned in the same session dies with it. A helper survives only if it escapes the session (`setsid`/`nohup setsid`, or a double-fork daemon). Not tested, but follows from the `getsid` filter above.
- Docs say only "closes when the command exits or a `popup.close` request is sent" (PL:310-311); the signal behaviour is not documented.

## 3. Gap / flicker

- The popup is drawn server-side into each client frame as `ClientShellPopupSurface`, an `Option` read from `state.popup_pane` on every frame (`src/server/client_shell.rs:503-545`; client border/title at `src/client/shell/composition.rs:583`). `close_popup_pane` sets `mode=Terminal` and requests a render (`popup.rs:24-31`). Renders are throttled to >= 16 ms apart (`src/app/mod.rs:39`).
- Between `close` returning and the next `open` being handled, the state has no popup. Whether a frame is emitted in that window depends on request latency (a fresh `herdr` CLI process spawn + socket round trip, typically several ms to tens of ms) vs the 16 ms render gate, so **at least one popup-less frame is likely but not guaranteed**; no code makes the swap atomic. Keystrokes in the gap go to the underlying tiled pane (popup input capture ends with `popup_pane`, `src/server/headless.rs:1204,2394`), so a typed key could leak to the source pane. Unmeasured.
- Also during that gap another client/keybinding could win the singleton (`ui_busy` for the helper).

## 4. State hand-off

- `--env KEY=VALUE` (socket `env` map) passes through to the new process (`panes.rs:246-260`; `PluginPaneOpenParams.env`, `src/api/schema/plugins.rs:458-459`). So `--env PIP_SOURCE=<pane> --env PIP_SIZE=...` works; new size is also known to the child from its PTY winsize.
- Protected keys are only stripped **from the caller-supplied env** (`panes.rs:260`, list at `panes.rs:356-369`) and then herdr **re-injects its own values** (`panes.rs:261-279`): `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR` (`env.rs:16-29`), `HERDR_SOCKET_PATH`, `HERDR_ENV=1`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ENTRYPOINT_ID`, `HERDR_PLUGIN_CONTEXT_JSON` (built fresh from live state at reopen time, `popup.rs`/`panes.rs:18`), and `HERDR_BIN_PATH`. So the child **does have the socket path and binary path**, hence can call `"$HERDR_BIN_PATH" plugin pane open --plugin "$HERDR_PLUGIN_ID" --entrypoint "$HERDR_PLUGIN_ENTRYPOINT_ID" ...` or speak JSON to `$HERDR_SOCKET_PATH` directly. It has **no pane id**: `HERDR_PANE_ID` is removed (`src/pane.rs:178-181,211-215`).
- Source pane id: the reopen recomputes `focused_pane_id` from live state (underlying tiled pane, since the popup takes no focus); if the PiP target differs from focus, carry it via `--env PIP_SOURCE=` (the old process can read it from its own env and re-pass it). Context JSON therefore cannot carry the target by itself.
- `HERDR_WORKSPACE_ID` / `HERDR_TAB_ID` are not set by the popup launcher (`OmitPane` only unsets the pane var, `pane.rs:211-215`); they are whatever the herdr server process env contributes, i.e. unreliable. Use CONTEXT_JSON.
- The reopen can pass `--width/--height` and reuse the same entrypoint, or a second entrypoint (e.g. `pip` vs `pip-resized`) whose manifest `[[panes]]` carries different default width/height (PL:322-330; `panes.rs:24-25`: request value wins over manifest). Skipping the picker is just the child checking `PIP_SOURCE`.

## 5. Hidden resize methods or flags: none

- Popup size lives in `PopupPaneState { width, height }` (`src/app/state.rs:18-23`), written only at spawn (`popup.rs:162,186`); a repo-wide search for assignments to `popup.width/height` finds none, and there is no `popup.resize`/`popup.set_size` method. Method list for popups is `popup.close` only (`src/api/schema.rs:131`, `src/api/schema/operations.rs:202`; SOCK:107).
- The `--width/--height` flags are hidden in clap help but accepted (`src/cli/plugin/bindings.rs:122-135`; test `:441-470`); "fullscreen" is also a hidden accepted placement string there (not investigated; not in the popup path).
- The only in-place size change is automatic: percent sizes and area clamping are re-resolved on each render and the PTY is resized, giving the child SIGWINCH (`src/ui/panes.rs:536-552`, `src/server/client_shell.rs:541-562`), but the stored spec never changes. So a cell-sized popup can shrink only if the terminal shrinks. A process cannot request a size through the PTY either (no herdr handling of xterm `CSI 8 t` was found in this pass; not exhaustively searched).

## 6. Clamps on --width / --height

- CLI parse (`src/popup_size.rs:17-30`): `N` -> u16 cells (0..=65535; `65536`/`bad` rejected), `N%` -> u8 percent, accepted 1..=100 (`0%`, `101%` rejected). `--width=0` is accepted (test `bindings.rs:462-465`). JSON/manifest strings must be percentages, numbers are cells (`popup_size.rs:41-47`).
- Resolution (`popup_size.rs:55-69`): outer = resolved size, `.max(6)` wide / `.max(4)` tall, then `.min(area)`. Default is area/2 with the same minimums. So min outer 6x4, max = whole pane surface (terminal area minus sidebar/tab bar; not larger than the screen). If the area itself is < 6x4 the open fails "terminal area too small for popup" (`popup.rs:148-152`).
- Child-visible PTY = outer - 2 rows; cols = outer - 2, minus one more when that is > 4 (`popup_size.rs:73-85`). Smallest popup: outer 6x4 -> PTY 4x2.

## Implications

- No in-place resize exists; self-resize = close then reopen. The popup process cannot do both: `popup.close` (or its own exit) kills its whole session within <= ~750 ms, and a reopen sent while it is still open gets `ui_busy`.
- Viable recipe: the popup spawns a detached helper in a new session (`setsid`) that calls `"$HERDR_BIN_PATH" plugin pane open --plugin "$HERDR_PLUGIN_ID" --entrypoint "$HERDR_PLUGIN_ENTRYPOINT_ID" --placement popup --width W --height H --env PIP_SOURCE=... ` in a retry loop on `ui_busy`/`popup already open`, then the popup exits (or calls `popup.close`). Simpler variant: the popup just exits and the helper retries until the open succeeds. All needed values (`HERDR_BIN_PATH`, `HERDR_SOCKET_PATH`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ENTRYPOINT_ID`) are in the child env; the helper inherits them.
- Expect a short popup-less window (likely <= one or two frames, unmeasured) and possible key leak into the source pane in that window; hide it by having the helper retry fast (no sleep before the first attempt) and, if it matters, probe live once before committing (`pip` plugin test: time `popup.close` -> next `open` success, observe screen capture).
- Hand state through `--env PIP_SOURCE=<pane-id>` (plus size/mode vars); `HERDR_PLUGIN_CONTEXT_JSON` is rebuilt at reopen and only knows the focused pane, so it is not a substitute. Do not rely on `HERDR_PANE_ID`/`HERDR_WORKSPACE_ID`.
- A design that avoids reopening is cheaper: pick the popup size once (via picker or per-source `--width/--height` from the launching action) and use a percent size if it should track the terminal. A second entrypoint with a different manifest size is an alternative to passing flags, but still needs the reopen.
- Clamps: min outer 6x4, max the pane surface; sizes outside are silently clamped, never an error (except area too small). Percent range 1..=100.
- Open question for a live probe: exact latency/visibility of the swap on macOS, and whether a `setsid` helper survives the session sweep as predicted.
