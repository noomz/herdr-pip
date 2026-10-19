---
title: How should a pinned split mirror behave?
type: grilling
status: closed
assignee: noomz
blocked_by: [09-spike-image-mirror.md]
---

## Question

The popup peek is modal, so the user pins it into a non-modal split pane mirror (`herdr plugin pane open --placement split --direction right|down --no-focus --target-pane`; a normal herdr pane, resizable with `pane.resize`, movable with `pane.move`). Decide: pin key and where the split opens (direction, size, which pane it splits); whether the pinned mirror follows tab/workspace switches (event hooks + `pane.move`) or stays put; one pinned mirror or many; what input the pinned pane takes when clicked/focused (jump to source, unpin, re-pick source); scale/fit inside the split (letterbox reuse); unpin/close; what happens when the source closes. Facts: [Does a kitty image render correctly inside a popup?](09-spike-image-mirror.md), [How should the peek look and behave?](03-peek-ux.md).

## Resolution

Out of scope (2026-10-19, user). The pinned split stood in for a non-modal PiP while the peek was a modal popup. [Build: plugin skeleton, launcher and socket client](10-build-skeleton.md) moved the peek to fork-herdr floats. A float is non-modal and draggable, takes input only when focused, shows on every tab, and hides while its source is focused. Decisions:

- Target: fork herdr is primary. Stock herdr 0.9.5 only needs the modal popup fallback to work (picker, mirror, resize saves scale, close).
- No pinned split on either. On stock herdr the popup stays modal. A tiled mirror that takes layout space would be a new feature, not this destination.
- Glossary: Peek redefined as a floating window (popup on herdr without floats).
