# herdr-pip

A [herdr](https://github.com/herdrdev/herdr) plugin that shows another
pane's live output in a small window over your current view, like
picture-in-picture. Pick a pane, keep working, and glance at it while an agent
runs.

- On herdr with floats, the peek is a **non-modal float**. You keep typing in
  your panes underneath, and herdr hides the float while its source pane is
  focused.
- On stock herdr 0.9.5 (no floats), the peek is a **modal popup**. It takes all
  input until you close it.

Terms used below (peek, source pane, picker, mirror) are defined in
[GLOSSARY.md](GLOSSARY.md).

## Install

Requires herdr 0.9.5 or later on macOS or Linux, and a Rust toolchain.

```sh
git clone https://github.com/noomz/herdr-pip
cd herdr-pip
cargo build --release
herdr plugin link .
```

`herdr plugin link` does not run a build step, so build before you link, and
run `cargo build --release` again after each update.

Check that herdr sees it:

```sh
herdr plugin list   # - herdr-pip (PiP) enabled
```

## Keybinding

Add this to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+shift+p"
type = "plugin_action"
command = "herdr-pip.peek"
description = "PiP: peek at a pane"
```

`prefix+shift+p` is free in herdr's defaults. If you already use it, try
`prefix+o`. Reload with `herdr server reload-config` or restart herdr.

You can also open a peek without a key:

```sh
herdr plugin action invoke herdr-pip.peek
```

## Use

The peek opens on the **picker**: every pane except the focused one, panes that
need attention first (`!` blocked, `*` done, then working, idle and plain
shells).

| Picker key | Action |
|---|---|
| type | filter by name or agent status |
| `↑`/`↓`, wheel, or `j`/`k` (empty filter) | move |
| Enter or click a row | peek at that pane |
| Esc | clear the filter, then close |
| `q` (empty filter) or click `[x]` | close |

After you choose a pane, the peek shows its **mirror**. The top row shows the
source and status, and a clickable `[x]`.

| Mirror key | Action |
|---|---|
| Enter or click the mirror | jump: focus the source pane |
| `p` or Tab | back to the picker |
| `+` / `-` | make the peek larger / smaller |
| `=` | reset the size |
| Esc, `q`, or click `[x]` | close |

The mirror is read-only: other keys do nothing, and nothing is sent to the
source pane.

- In a float, a jump keeps the peek. herdr hides it while the source is focused
  and shows it again when you leave. In a popup, a jump closes the peek.
- If the source pane closes, the peek goes back to the picker with
  `source closed`.

### Size

The peek has the source pane's shape at a scale of 50% (20–100%, steps of 10),
clamped to 90%×85% of the tab. `+`, `-` and `=` change the scale. It is saved
in `$XDG_STATE_HOME/herdr-pip/scale` (default `~/.local/state/herdr-pip/scale`)
and applies to every later peek.

A float resizes at once. A popup cannot resize: it keeps the size it opened
with (60%×50% when it opens on the picker), the keys save the scale for the
next peek, and the top row shows `resize needs floats`. The picker is always
60%×50%.

## Image mirror and text mirror

herdr-pip draws the mirror in one of two ways. It checks when the peek starts.

- **Image mirror**, when your terminal supports the kitty graphics protocol
  (for example kitty or Ghostty) and herdr passes kitty images through (the
  default). The whole source screen is drawn as a scaled-down picture, up to 10
  times per second and only when it changes.
- **Text mirror**, otherwise. The mirror shows real text: the source's last
  rows and left columns, as many as fit.

The image mirror uses fixed dark colours, because the peek cannot read your
herdr theme. With a light theme, the picture is still dark.

### Font

The image mirror draws text with a bundled copy of JetBrains Mono Regular
(licence: [assets/JetBrainsMono-OFL.txt](assets/JetBrainsMono-OFL.txt)).
Characters that the font does not have are drawn as boxes. To use another
monospace font, set `PIP_FONT` in the herdr server's environment:

```sh
PIP_FONT=/path/to/Font.ttf
```

If the font cannot be loaded, the peek uses the bundled font and shows the
error in the top row.

## Environment

| Variable | Effect |
|---|---|
| `PIP_FONT` | Font file (TTF/OTF) for the image mirror. |
| `PIP_SOURCE` | Pane id, for example `w1:p3`. `herdr-pip launch` with this set skips the picker. |
| `XDG_STATE_HOME` | Where the scale file is saved. |

To open a peek on a known pane from a script inside herdr:

```sh
HERDR_PLUGIN_ID=herdr-pip PIP_SOURCE=w1:p3 \
  ~/path/to/herdr-pip/target/release/herdr-pip launch
```

## Licence

MIT. The bundled font is under the SIL Open Font License 1.1.
