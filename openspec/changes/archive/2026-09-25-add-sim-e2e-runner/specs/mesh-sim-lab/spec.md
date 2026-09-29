## ADDED Requirements

### Requirement: Health e2e is a gated command

The sim lab SHALL provide `deploy/sim/e2e/run.sh health` that checks
mgmt SSH, the sim-guest marker, 802.11s ESTAB, and overlay `10.254`
ping. Partitioning vwifi SHALL fail overlay ping while mgmt SSH still
works. The runner SHALL NOT close beads.

#### Scenario: Health pass

- GIVEN the q35 guests are running and vwifi is up
- WHEN an operator runs `deploy/sim/e2e/run.sh health`
- THEN SSH to 10.99.0.10–14 succeeds in BatchMode
- AND each node guest has `/etc/mjolnir/sim-guest`
- AND 802.11s is ESTAB between node-a and node-b
- AND `10.254` ping a↔b succeeds

#### Scenario: Partition fails overlay not SSH

- GIVEN health preconditions
- WHEN vwifi-server is stopped
- THEN overlay `10.254` ping fails
- AND mgmt SSH still succeeds
- AND the runner restores vwifi before exit
