## ADDED Requirements

### Requirement: Revision-checked target-owned apply

The target SHALL serialize managed configuration changes, reject stale revisions
before mutation, and bind an idempotent transaction ID to one complete typed plan.

#### Scenario: Concurrent stale preview
- GIVEN two controllers preview the same revision
- WHEN the first commits and the second applies its stale preview
- THEN the second is refused before mutation and receives the current revision

#### Scenario: Reused ID
- GIVEN a transaction ID already names a plan
- WHEN a request reuses that ID
- THEN the same plan returns its recorded state and a different plan is refused

### Requirement: Durable recovery before destructive work

The system SHALL durably record complete rollback inputs before mutation and
recover interrupted uncommitted transactions after process or node restart.
Uncertain, corrupt or incomplete recovery SHALL report recovery-required and
block further managed changes rather than imply a restored network.

#### Scenario: Reboot during apply
- GIVEN a prepared transaction has begun changing configuration
- WHEN the node reboots before a durable commit
- THEN boot recovery restores its snapshot and independently verifies restoration
  or records recovery-required with retained artifacts

#### Scenario: Storage failure
- GIVEN snapshot or journal persistence fails
- WHEN the target attempts to advance the transaction
- THEN it does not perform a mutation that lacks durable recovery inputs

### Requirement: Honest outcome and connectivity evidence

Receipts SHALL distinguish committed, restored and recovery-required configuration
states from connectivity observations and their vantage. Controller loss SHALL
not cancel target recovery. Local probes SHALL NOT establish downstream internet.

#### Scenario: Lost controller and incomplete client evidence
- GIVEN the controller disconnects while the target completes a healthy apply
- WHEN it reconnects and retrieves the transaction
- THEN it receives the durable outcome with client internet unknown unless a
  qualifying downstream client-path observation was recorded

#### Scenario: Failed restoration
- GIVEN rollback copies the old files but management does not recover
- WHEN restoration checks fail
- THEN the outcome is recovery-required, not restored

