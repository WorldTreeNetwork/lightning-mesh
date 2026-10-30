# admin-stdout-views (source swap)

Deltas for one canvas switching allowlisted sources. Living spec pasted
from `openspec/specs/admin-stdout-views/spec.md` at preparation.

## ADDED Requirements

### Requirement: Source swap

The allowlist SHALL include a source named `netstat` whose argv is
`["/usr/sbin/netstat", "-ib"]`. The canvas SHALL show a control for each
allowlisted source name. Activating a name SHALL cancel and reap any
in-flight child for that canvas and SHALL fill the well from that source.
The selected name SHALL be indicated. Opening the canvas with no selection
SHALL still fill `ifconfig`. A swap SHALL use the same timeout and capture
cap as `ifconfig`. On a host that is not macOS, `netstat` SHALL show
unsupported and SHALL NOT spawn.

#### Scenario: Operator switches to netstat

- GIVEN Lightning Admin on macOS with one stdout-view canvas showing `ifconfig`
- WHEN the operator activates `netstat`
- THEN the well shows that `netstat -ib` transcript or a visible failure,
  and the indicated source is `netstat`

#### Scenario: Switch back

- GIVEN the canvas source is `netstat`
- WHEN the operator activates `ifconfig`
- THEN the well shows the `ifconfig` transcript or a visible failure,
  and the indicated source is `ifconfig`

#### Scenario: Swap drops the previous child

- GIVEN a fill still running
- WHEN the operator activates the other allowlisted source
- THEN the previous child is terminated and reaped, and its later output
  does not replace the new source's result

## MODIFIED Requirements

### Requirement: One canvas in this landing
<!-- base: 4634f71868bb844827bdb4c18ceadbaabb2e4c9f openspec/specs/admin-stdout-views/spec.md -->

Lightning Admin SHALL provide one stdout-view canvas until
`add-crystal-grid`. Opening another canvas, and a spatial grid, diorama,
or 3d layout, SHALL wait on that change. That one canvas SHALL switch
among allowlisted sources. A source switch SHALL NOT open a second window.

#### Scenario: Default open is a single canvas

- GIVEN Lightning Admin on macOS
- WHEN the operator opens a stdout-view canvas with the default source
- THEN one canvas window exists and its well fills from `ifconfig`

#### Scenario: Swap stays one window

- GIVEN one open stdout-view canvas
- WHEN the operator selects another allowlisted source
- THEN the same window remains and no second canvas is created
