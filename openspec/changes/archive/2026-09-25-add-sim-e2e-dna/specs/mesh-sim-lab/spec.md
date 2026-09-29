## ADDED Requirements

### Requirement: DNA e2e is a gated command

`deploy/sim/e2e/run.sh dna` SHALL run the RFC 4436 INIT-REBOOT probe
and SHALL NOT close `mjolnir-mesh-sz9.1`.

#### Scenario: Observation recorded

- GIVEN a STA with a lease
- WHEN `run.sh dna` finishes successfully
- THEN keep vs hole is logged
- AND `sz9.1` remains open
