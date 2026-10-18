---
title: Build: text mirror
type: task
status: open
assignee:
blocked_by: [10-build-skeleton.md]
---

## Question

`terminal session observe` at the source's full size → `vt100` grid → raw ANSI row-diff paint of bottom rows / left columns, 30 fps cap, restart on SIGWINCH, `terminal.closed` → picker with notice. Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md).
