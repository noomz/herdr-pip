---
title: Build: text mirror
type: task
status: closed
assignee: noomz
blocked_by: [10-build-skeleton.md]
---

## Question

`terminal session observe` at the source's full size → `vt100` grid → raw ANSI row-diff paint of bottom rows / left columns, 30 fps cap, restart on SIGWINCH, `terminal.closed` → picker with notice. Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md).

## Resolution

Built in `src/mirror.rs` (commits `5455a4d`, `37f9bd3`) and verified live on 2026-10-18 in the `mac_jr` VM:

- `herdr terminal session observe <pane> --cols C --rows R` at the source's full size feeds a `vt100` grid on a reader thread. The grid is one column wider than the source, so `rows_formatted` never joins wrapped rows and each row's bytes use only relative moves within that row.
- Paint: changed rows only, each placed with CUP and cleared with `2K`, in one `?2026` batch, at most 30 fps. The peek used 0.3% CPU mirroring a 4 lines/s source.
- **Changed from the decision:** the mirror ends at the source's content (its last non-blank row or the cursor row, whichever is lower), not at the last screen row. Plain bottom-anchoring showed only blank rows for a fresh shell.
- **New with floats:** a float is non-modal, so the source can be resized mid-peek. The peek re-reads the source size every 2 s and on SIGWINCH, and restarts the observer when the size changed (verified: 18 → 26 rows).
- `terminal.closed` or EOF stops the mirror, keeps the last frame and shows a `source closed` notice. The picker is ticket 12.
