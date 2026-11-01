# admin-stdout-views (crystal grid)

Deltas for more than one stdout well. Living requirement pasted from
`openspec/specs/admin-stdout-views/spec.md` at preparation.

## ADDED Requirements

### Requirement: Grid of stdout wells

Lightning Admin SHALL show at least two stdout-view wells at once, and
SHALL NOT open more than four. Opening another well when four are open
SHALL show a visible refusal and SHALL NOT create a window. Opening a
well SHALL mint a new `canvas-*` window label. It SHALL NOT focus an
existing well instead. The well id for fill and cancel SHALL be that
window's label and SHALL NOT be an IPC argument. Each well SHALL keep
its own allowlisted source and its own in-flight child. A fill or
source swap in one well SHALL NOT cancel another well's child. Closing
a well SHALL reap only that well's child. Focusing a well SHALL NOT
close another well. Each well SHALL be its own owner-drawn window, with
the same chrome as the current canvas, including Control-drag move and
Option-drag resize. Those gestures SHALL NOT snap the well back onto a
cell. A new well's top-left SHALL be one cell step to the right of the
most recently opened well, wrapping down when that position would leave
the screen, and clamped on screen. Positions last for the Admin session
only and SHALL NOT be written to a layout file. A 3d or diorama engine
SHALL NOT be used.

#### Scenario: Two wells at once

- GIVEN Lightning Admin on macOS
- WHEN the operator opens a second stdout-view well
- THEN two wells are visible
- AND the second well has a new `canvas-*` label
- AND each can show `ifconfig` or `netstat -ib` independently

#### Scenario: A fill in one well leaves the other child running

- GIVEN well A has a fill still running
- WHEN well B starts a fill or swaps source
- THEN A's child is not terminated
- AND A's result still paints in A

#### Scenario: Closing one well reaps only its child

- GIVEN well A has a fill still running and well B is open
- WHEN the operator closes well A
- THEN only A's child is reaped
- AND well B's in-flight fill can still complete

#### Scenario: Fifth well is refused

- GIVEN four stdout wells are open
- WHEN the operator asks for another
- THEN no fifth window is created
- AND the refusal is visible

#### Scenario: Focus keeps the other well

- GIVEN two open stdout wells
- WHEN the operator focuses one
- THEN the other well remains open with its transcript

#### Scenario: One well fails and the other stays

- GIVEN two wells with transcripts
- WHEN one well's fill times out or is refused
- THEN that well shows the failure
- AND the other well's transcript is unchanged

## RENAMED Requirements

### Requirement: One canvas in this landing
<!-- base: 053357113266840466e6d2442a96b1202971e215 openspec/specs/admin-stdout-views/spec.md -->

**Renamed to:** Canvas count

Lightning Admin SHALL allow more than one stdout-view canvas, up to
the grid maximum. A source switch on one canvas SHALL NOT open or
close another canvas. Each canvas SHALL keep its own selected source.
The default source for a canvas opened with no selection SHALL remain
`ifconfig`.

#### Scenario: Default open is a single canvas

- GIVEN Lightning Admin on macOS
- WHEN the operator opens a stdout-view canvas with the default source
- THEN that canvas's well fills from `ifconfig`

#### Scenario: Swap stays inside one well

- GIVEN two open stdout-view canvases
- WHEN the operator selects another allowlisted source on one of them
- THEN only that canvas changes source
- AND the other canvas keeps its source and transcript
