## ADDED Requirements

### Requirement: Lightning settings are the source of truth

Operator knobs for the client AP (SSID, encryption, key) and the WPS
WAN-LAN SSH window timeout SHALL live in `/etc/config/mjolnir` (`radio`,
`wan_admin`). `mjolnir-apply` SHALL be the only projector of those knobs
onto OpenWrt UCI `wireless`. A staged `wireless.env` SHALL update the
mjolnir store, then be wiped; it SHALL NOT be the live source after apply.

#### Scenario: Env seeds the store then wireless

- GIVEN staged `CLIENT_SSID='Lightning Mesh'` and `RUN_WIRELESS=1`
- WHEN `mjolnir-apply` completes OK
- THEN `mjolnir.radio.ssid` is `Lightning Mesh` and
  `wireless.clientap.ssid` equals that value

#### Scenario: Store wins on a later apply without env

- GIVEN `mjolnir.radio.ssid` is `Lightning Mesh` and no staged `wireless.env`
- WHEN `mjolnir-apply` runs with `RUN_WIRELESS=1`
- THEN `wireless.clientap.ssid` is `Lightning Mesh`, not the factory default

#### Scenario: Factory defaults when unset

- GIVEN no `mjolnir.radio` section on a node
- WHEN apply ensures settings
- THEN `mjolnir.radio.ssid` is ⚡ and `mjolnir.radio.encryption` is `none`

### Requirement: WAN-LAN SSH timeout is a setting

`mjolnir-wan-admin` SHALL take its arm duration from
`mjolnir.wan_admin.timeout` (seconds). If the option is missing or not a
positive integer it SHALL fall back to 900.

#### Scenario: Custom timeout

- GIVEN `mjolnir.wan_admin.timeout=120`
- WHEN WPS arms the window
- THEN the sleeper disarms after 120 seconds

#### Scenario: Missing option

- GIVEN no `wan_admin` timeout option
- WHEN WPS arms the window
- THEN the duration is 900 seconds
