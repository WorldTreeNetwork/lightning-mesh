## ADDED Requirements

### Requirement: WPS arms WAN-LAN SSH

A short press of the WPS button SHALL toggle a time-boxed nft accept of
TCP/22 on the `wan` zone, sourced only from prefixes currently configured
on the WAN interface. The window SHALL last 15 minutes unless closed
earlier. The rule SHALL NOT be written to UCI. LuCI and hello.mesh SHALL
remain closed on WAN. Stock hostapd/wpa_supplicant WPS-PBC SHALL NOT start
as a result of that press.

#### Scenario: Press opens SSH from the WAN LAN

- GIVEN a node with WAN lease `192.168.0.15/24` and an authorized pubkey
- WHEN the operator presses WPS
- THEN `ssh root@192.168.0.15` from `192.168.0.0/24` succeeds within a few
  seconds and the WPS LED blinks

#### Scenario: Prefix, not the internet

- GIVEN the window is armed
- WHEN a host not in the WAN connected prefixes connects to TCP/22
- THEN the connection is refused (wan input reject)

#### Scenario: Second press or reboot closes

- GIVEN an armed window
- WHEN the operator presses WPS again, or the node reboots, or 15 minutes
  elapse
- THEN WAN TCP/22 is refused again and the WPS LED stops blinking

### Requirement: Operator keys merge on apply

`mjolnir-apply` SHALL merge staged operator SSH pubkeys into
`/etc/dropbear/authorized_keys` without deleting keys already present.
Dropbear SHALL keep listening on TCP/22 (firewall, not bind, is the WAN
gate).

#### Scenario: Apply adds a missing workstation key

- GIVEN a staged pubkey that is not on the node
- WHEN `mjolnir-apply` completes OK
- THEN that line is present in `/etc/dropbear/authorized_keys`

#### Scenario: Existing keys survive

- GIVEN a node that already has extra authorized keys
- WHEN apply merges the staged list
- THEN the extra keys remain
