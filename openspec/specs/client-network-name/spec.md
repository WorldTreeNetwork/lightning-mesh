# client-network-name

The fleet client AP SSID is a public radio network name. It is not a
guild. Folded from `add-client-network-name` (2026-08-31) and
`add-lightning-admin-network-name` (2026-09-04).

Code: `deploy/openwrt/setup-wireless.sh`, `deploy/openwrt/files/usr/sbin/mjolnir-apply`,
`admin/` (writer of the same apply path).

## Requirements

### Requirement: Client SSID is a network name

The fleet client AP SSID SHALL be a public radio network name. It SHALL be
written only through `mjolnir-apply` running `setup-wireless.sh` (or an
equivalent explicit radio write), including a Lightning Admin Apply that
drives that same path. Docs and operator copy SHALL call it a
network name, not a guild. Applying a new SSID SHALL NOT write
`identikey-log` and SHALL NOT mint keyspace membership.

#### Scenario: Apply sets the name phones see

- GIVEN staged `CLIENT_SSID` and `RUN_WIRELESS=1`
- WHEN `mjolnir-apply` completes OK
- THEN `wireless.clientap.ssid` equals that value

#### Scenario: No guild log on radio write

- GIVEN an apply that changes `CLIENT_SSID`
- WHEN the apply completes
- THEN no `keyspace.*` op is appended to `identikey-log`

#### Scenario: Association is not membership

- GIVEN a phone that associates to the client SSID
- WHEN no identikey login has occurred
- THEN the holder is not a keyspace member

#### Scenario: Admin is a writer of the same path

- GIVEN Lightning Admin Apply of a network name
- WHEN the fleet apply completes OK for a node
- THEN that node was written through `mjolnir-apply` / `setup-wireless.sh`,
  not a live UCI SSH mutation

### Requirement: Factory default is open ⚡

A new node with no `CLIENT_SSID` override SHALL beacon SSID ⚡ (U+26A1
HIGH VOLTAGE SIGN, UTF-8, no variation selector) with
`encryption=none`. A live fleet override in `wireless.env` SHALL be
honored until an operator apply changes it.

#### Scenario: Fresh box without override

- GIVEN `setup-wireless.sh` and no `CLIENT_SSID` in the environment
- WHEN wireless is configured
- THEN the client AP SSID is ⚡ and encryption is none

#### Scenario: Live fleet override survives

- GIVEN `CLIENT_SSID='Lightning Mesh'` in the staged env
- WHEN `mjolnir-apply` runs with `RUN_WIRELESS=1`
- THEN the client AP SSID is `Lightning Mesh`, not the factory default

### Requirement: Lightning Admin writes the fleet network name

Lightning Admin SHALL expose a control labeled network name (not guild)
that sets the fleet client AP SSID. Confirming Apply SHALL stage
`CLIENT_SSID` and invoke the existing fleet radio apply
(`update-fleet.sh --wireless` → `mjolnir-apply` with `RUN_WIRELESS=1`).
It SHALL NOT write UCI over a live SSH session, SHALL NOT append
`identikey-log`, and SHALL NOT mint keyspace membership. Open
(`encryption=none`) SHALL remain representable. An empty name SHALL NOT
apply.

#### Scenario: Operator sets the name from Admin

- GIVEN Lightning Admin on a workstation that can SSH the overlay inventory
- AND a non-empty network name within the 32-octet SSID limit
- WHEN the operator confirms Apply
- THEN each reachable node in `fleet-nodes.conf` is applied through
  `mjolnir-apply` with that `CLIENT_SSID`
- AND `wireless.clientap.ssid` on a successful node equals that value

#### Scenario: Empty name does not apply

- GIVEN the network name field is empty
- WHEN the operator is at the Apply control
- THEN Apply is disabled and no `mjolnir-apply` runs

#### Scenario: Failed apply keeps the previous name

- GIVEN a fleet apply that health-gate fails on a node
- WHEN Admin shows the result
- THEN that node has rolled back (or reports FAILED with nothing
  changed) and Admin reports halt/skip rather than claiming success

#### Scenario: Admin apply is not a guild op

- GIVEN an Admin apply that changes `CLIENT_SSID`
- WHEN the apply completes
- THEN no `keyspace.*` op is appended to `identikey-log`
- AND the UI copy does not call the field guild
