---
title: How should a pinned split mirror behave?
type: grilling
status: open
assignee:
blocked_by: [09-spike-image-mirror.md]
---

## Question

The popup peek is modal, so the user pins it into a non-modal split pane mirror (`herdr plugin pane open --placement split --direction right|down --no-focus --target-pane`; a normal herdr pane, resizable with `pane.resize`, movable with `pane.move`). Decide: pin key and where the split opens (direction, size, which pane it splits); whether the pinned mirror follows tab/workspace switches (event hooks + `pane.move`) or stays put; one pinned mirror or many; what input the pinned pane takes when clicked/focused (jump to source, unpin, re-pick source); scale/fit inside the split (letterbox reuse); unpin/close; what happens when the source closes. Facts: [Does a kitty image render correctly inside a popup?](09-spike-image-mirror.md), [How should the peek look and behave?](03-peek-ux.md).
