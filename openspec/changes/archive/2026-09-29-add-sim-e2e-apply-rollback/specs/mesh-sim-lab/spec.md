## ADDED Requirements

### Requirement: Apply-rollback e2e is a gated command

`deploy/sim/e2e/run.sh apply-rollback` SHALL run `mjolnir-apply` on a
sim guest that presents `/etc/mjolnir/sim-guest`. The run SHALL
snapshot current allowlisted UCI, stage a UCI mutation that fails
closed before commit, and require the snapshot to be restored. The
command SHALL fail if the result is not `ROLLED_BACK`, if UCI still
contains the staged change, or if overlay `10.254` ping a↔b does not
succeed after restore. The runner SHALL NOT close beads. Metal apply
remains a separate hardware gate. The harness SHALL NOT bounce
sim radios (`RUN_WIRELESS` / `wifi reload`) — dedicated hostapd is
the client AP.

#### Scenario: Bad UCI rolls back

- GIVEN node-a is a marked sim guest with overlay ping to node-b
- WHEN `deploy/sim/e2e/run.sh apply-rollback` runs
- THEN `mjolnir-apply` writes `ROLLED_BACK`
- AND the staged UCI mutation is absent
- AND overlay `10.254` ping a↔b succeeds
- AND `lpv` / `z3th` remain open

#### Scenario: Marker still required

- GIVEN a node without `/etc/mjolnir/sim-guest`
- WHEN the sim apply-rollback profile would write UCI
- THEN it aborts before mutation
