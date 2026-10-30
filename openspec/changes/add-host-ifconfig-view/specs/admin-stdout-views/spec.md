# admin-stdout-views (ifconfig fill)

Deltas for this landing. Parent contract still unfolder.

## ADDED Requirements

### Requirement: ifconfig fill bounds

The `ifconfig` source SHALL spawn argv `["/sbin/ifconfig"]` with a **5 second**
timeout and a **65536 byte** capture cap. Combined stdout and stderr SHALL be
shown as text. Hitting the cap SHALL set truncated and SHALL NOT wait for the
child to exit beyond the timeout. Timeout SHALL be a failure. Non-macOS SHALL
be unsupported with no spawn.

#### Scenario: Default open fills ifconfig

- GIVEN Lightning Admin on macOS with `/sbin/ifconfig`
- WHEN the operator opens the stdout-view canvas
- THEN the well contains that process's combined transcript or a visible
  failure, and is not left empty without a state

#### Scenario: Cap truncates

- GIVEN a fill that writes more than 65536 bytes
- WHEN capture stops
- THEN the well is truncated and the truncation is visible

#### Scenario: Timeout fails

- GIVEN a child that does not exit within 5 seconds
- WHEN the timeout fires
- THEN the well shows failure and the child is killed and reaped
