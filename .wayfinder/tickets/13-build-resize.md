---
title: Build: reopen helper, resize keys and remembered scale
type: task
status: open
assignee:
blocked_by: [08-spike-reopen.md, 10-build-skeleton.md]
---

## Question

Detached reopen helper, size formula and clamps, `+`/`-`/`=`, `p`/Tab reopen at picker size, scale persisted in `$XDG_STATE_HOME/herdr-pip/scale`. Shape depends on the spike outcome in [Does the reopen helper resize the popup acceptably?](08-spike-reopen.md). Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md).

## Context added after [Build: picker](12-build-picker.md)

The fork's `float.resize` (plus `float.set_hide_rule`) resizes a float peek in place: the picker already uses it to switch to ratio size and back to `60%`×`50%`. In a float, `+`/`-`/`=` can call `float.resize` directly with no reopen, no flash, and no key-leak gap. The detached reopen helper is needed only for the modal popup fallback (herdr without floats), where the picker currently switches in place at picker size. Decide whether the popup fallback still earns the helper.
