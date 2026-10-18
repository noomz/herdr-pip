---
title: Build: reopen helper, resize keys and remembered scale
type: task
status: closed
assignee: noomz
blocked_by: [08-spike-reopen.md, 10-build-skeleton.md]
---

## Question

Detached reopen helper, size formula and clamps, `+`/`-`/`=`, `p`/Tab reopen at picker size, scale persisted in `$XDG_STATE_HOME/herdr-pip/scale`. Shape depends on the spike outcome in [Does the reopen helper resize the popup acceptably?](08-spike-reopen.md). Decisions: [How should the peek look and behave?](03-peek-ux.md), [Which language and render architecture?](04-architecture.md), [How should a small ratio popup fit its source?](05-small-popup-fit.md).

## Context added after [Build: picker](12-build-picker.md)

The fork's `float.resize` (plus `float.set_hide_rule`) resizes a float peek in place: the picker already uses it to switch to ratio size and back to `60%`×`50%`. In a float, `+`/`-`/`=` can call `float.resize` directly with no reopen, no flash, and no key-leak gap. The detached reopen helper is needed only for the modal popup fallback (herdr without floats), where the picker currently switches in place at picker size. Decide whether the popup fallback still earns the helper.

## Resolution

Decided with the user, 2026-10-18: **float-only resize, no reopen helper.** The detached `setsid` helper from [Does the reopen helper resize the popup acceptably?](08-spike-reopen.md) is not built.

- **Scale:** one global percent in `$XDG_STATE_HOME/herdr-pip/scale` (falls back to `~/.local/state`), default 50, `+`/`-` step 10 within 20–100, `=` resets to 50 and refits to the source's current size. A missing or unparsable file reads as 50; out-of-range values clamp. Code: `src/scale.rs`.
- **Float:** `+`/`-`/`=` in the mirror resize the float in place (`float.set_hide_rule` + `float.resize`, the picker's retarget call). No reopen, no flash, no key-leak gap.
- **Popup fallback:** the keys still change and save the scale, which applies from the next peek. The top row shows `<n>% · resize needs floats`. A failed save shows `scale not saved: …` instead.
- **Uses of the scale:** `launch` with `PIP_SOURCE`, and choosing a source in the picker. The picker stays `60%`×`50%`. Pressing `+` at 100 or `-` at 20 does nothing.
- **Deviation from [How should a small ratio popup fit its source?](05-small-popup-fit.md):** that ticket said the text mirror ignores the scale and uses 90%×85%. Builds 10–12 already gave the text mirror ratio size, so the scale applies to the text mirror too.

Verified live on 2026-10-18 in the `mac_jr` VM (fork herdr 0.9.5): a real float `f13` on source `w1:p7`, driven through a peek in tiled pane `w1:p8` with `HERDR_FLOAT_ID=f13` and `pane.send_text`; sizes read from `float.list`.

| Keys | Saved scale | Float size |
|---|---|---|
| start | none | 30×12 |
| `+` | 60 | 61×13 |
| `+`×5 | 100 | 90×21 (clamped to 90%×85%) |
| `-`×9 | 20 | 30×8 (minimum) |
| `=` | 50 | 51×12 |

- **Popup path** (no `HERDR_FLOAT_ID`): `-` saved 60, and the top row read `~/3/w1:p7 · 60% · resize needs floats`.
- **Remembered:** `launch` with scale 60 opened float `f14` at 61×13, the same size as `+` gave above.
- Unit tests: scale stepping, limits, reset, and parse fallback.
