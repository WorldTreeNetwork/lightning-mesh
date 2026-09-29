## ADDED Requirements

### Requirement: Fade e2e is a gated command

`deploy/sim/e2e/run.sh fade` SHALL increase vwifi distance until overlay
`10.254` ping fails, then restore coordinates and require reconvergence.
The run SHALL fail if overlay stays up. Killing vwifi-server is the
health partition, not this suite.

#### Scenario: Distance fade drops overlay

- GIVEN overlay ping a↔b works
- WHEN vwifi-ctrl moves guests far apart
- THEN overlay ping fails while mgmt SSH lives
- AND restoring coordinates restores overlay ping
