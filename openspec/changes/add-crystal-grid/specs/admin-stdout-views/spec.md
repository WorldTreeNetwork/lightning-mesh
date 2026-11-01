# admin-stdout-views (crystal grid)

Deltas for more than one stdout well. Living requirement pasted from
`openspec/specs/admin-stdout-views/spec.md` at preparation.

## ADDED Requirements

### Requirement: Grid of stdout wells

Lightning Admin SHALL show at least two stdout-view wells at once.
Each well SHALL keep its own allowlisted source and its own in-flight
child. Focusing a well SHALL NOT close another well. A source swap
SHALL affect only the well whose control was activated. Each well SHALL be its own owner-drawn window, with the same chrome
as the current canvas. The windows SHALL be placed in a grid for the
Admin session only. Positions SHALL NOT be written to a layout file.
A 3d or diorama engine SHALL NOT be used.

#### Scenario: Two wells at once

- GIVEN Lightning Admin on macOS
- WHEN the operator opens a second stdout-view well
- THEN two wells are visible
- AND each can show `ifconfig` or `netstat -ib` independently

#### Scenario: Focus keeps the other well

- GIVEN two open stdout wells
- WHEN the operator focuses one
- THEN the other well remains open with its transcript

## MODIFIED Requirements

### Requirement: One canvas in this landing
<!-- base: 053357113266840466e6d2442a96b1202971e215 openspec/specs/admin-stdout-views/spec.md -->

Lightning Admin SHALL allow more than one stdout-view canvas. A source
switch on one canvas SHALL NOT open or close another canvas. Each
canvas SHALL keep its own selected source. The default source for a
canvas opened with no selection SHALL remain `ifconfig`.

#### Scenario: Default open is a single canvas

- GIVEN Lightning Admin on macOS
- WHEN the operator opens a stdout-view canvas with the default source
- THEN that canvas's well fills from `ifconfig`

#### Scenario: Swap stays inside one well

- GIVEN two open stdout-view canvases
- WHEN the operator selects another allowlisted source on one of them
- THEN only that canvas changes source
- AND the other canvas keeps its source and transcript
