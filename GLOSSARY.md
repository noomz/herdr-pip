# herdr-pip

A herdr plugin that lets you glance at another pane's live output in a floating
window without leaving the current view.

## Language

**Peek**:
One open floating window (or a modal popup on herdr without floats) showing a live, read-only view of a single source pane.
_Avoid_: PiP window, preview

**Source pane**:
The pane whose output a peek shows.
_Avoid_: target, observed pane

**Picker**:
The list of candidate source panes shown in the peek before or between peeks.
_Avoid_: chooser, selector, menu

**Mirror**:
The area of the peek that reproduces the source pane's screen, as an image mirror or a text mirror.
_Avoid_: preview, viewport

**Image mirror**:
A mirror that shows the whole source pane scaled down to fit the peek, as a picture.
_Avoid_: thumbnail, screenshot

**Text mirror**:
A mirror that shows the source pane's bottom rows and left columns as real text, used when an image mirror is unavailable.
_Avoid_: crop view, fallback mode

**Jump**:
Leaving a peek by moving focus to its source pane.
_Avoid_: go to, switch, focus
