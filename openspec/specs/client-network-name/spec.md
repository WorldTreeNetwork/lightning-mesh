# client-network-name

The fleet client AP SSID is a public radio network name. It is not a
guild. Folded from `add-client-network-name` (2026-08-31).

Code: `deploy/openwrt/setup-wireless.sh`, `deploy/openwrt/files/usr/sbin/mjolnir-apply`.

## ADDED Requirements

### Requirement: Client SSID is a network name

The fleet client AP SSID SHALL be a public radio network name. It SHALL be
written only through `mjolnir-apply` running `setup-wireless.sh` (or an
equivalent explicit radio write). Docs and operator copy SHALL call it a
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
