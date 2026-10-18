---
title: What does the terminal observe stream deliver?
type: research
status: closed
assignee: research-agent
blocked_by: []
---

## Question

For `herdr terminal session observe <target>` and its socket method on herdr 0.9.5:

- Socket method name, request params, and the exact `terminal.frame` record
  shape. Is a frame a full-screen snapshot (repaint) or raw incremental PTY
  bytes? Is the first frame a full repaint?
- What do `--cols` / `--rows` mean for a read-only observer: does the server
  render the source at that viewport size for the observer only (no source
  resize), or crop, or ignore? This decides whether the popup can show the
  source at popup size without its own terminal emulator.
- Frame rate / coalescing, behavior for quiet panes, alt-screen apps,
  scrollback, cursor, and stream end (`terminal.closed`, 30 s stall rule).
- If frames are raw bytes at source size: what is the cheapest way to show
  them in a smaller viewport (a Rust VT parser crate like `vt100`, or herdr's
  own libghostty-vt?) and its rough CPU cost.

## Resolution

Observe is not a JSON socket method: the CLI `herdr terminal session observe <target> [--cols N --rows N]` uses the binary client protocol and prints NDJSON `terminal.frame {seq, encoding:"ansi", width, height, full, bytes(base64)}` and `terminal.closed {reason}`. Frames are server-rendered ANSI repaints (first is full, starts with ESC[2J; later ones diffs, but sync-output apps yield repeated ~20 KB full frames), not raw PTY bytes. `--cols/--rows` never resize the source (verified live and in code); the server crops the source viewport from the top-left at that size, with no reflow or bottom anchoring. So the popup needs no emulator for mirroring, but cannot show the bottom of a taller pane. Quiet panes emit nothing; 30 s write stall drops the observer. Details and citations: [research/01-observe-stream.md](../research/01-observe-stream.md).
