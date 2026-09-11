# wan-lan-admin

WPS short-press arms a prefix-boxed nft accept of TCP/22 on the WAN zone.
The prefixes are whatever is currently on the WAN iface (a house CPE LAN,
or an ISP segment if WAN is a direct lease). The rule is not written to
UCI; overlay remains the management plane. Folded from `add-wps-wan-admin`
(2026-09-10).

Code: `deploy/openwrt/files/usr/sbin/mjolnir-wan-admin`,
`deploy/openwrt/files/etc/rc.wps/00-mjolnir-wan-admin`,
`deploy/openwrt/files/usr/sbin/mjolnir-apply`.

## Requirements

### Requirement: WPS arms WAN-LAN SSH

A short press of the WPS button SHALL toggle a time-boxed nft accept of
TCP/22 on the `wan` zone, sourced only from prefixes currently configured
on the WAN interface. The window SHALL last `mjolnir.wan_admin.timeout`
seconds (default 900) unless closed earlier. The rule SHALL NOT be written
to UCI. LuCI and hello.mesh SHALL remain closed on WAN. Stock
hostapd/wpa_supplicant WPS-PBC SHALL NOT start as a result of that press.

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
- WHEN the operator presses WPS again, or the node reboots, or
  `mjolnir.wan_admin.timeout` seconds elapse (900 if unset)
- THEN WAN TCP/22 is refused again and the WPS LED stops blinking

#### Scenario: Apply or fw4 reload closes the window

- GIVEN an armed window
- WHEN `mjolnir-apply` reloads firewall, or `fw4 reload` runs
- THEN WAN TCP/22 is refused for new connections (the drop is the nft
  rule vanishing, not an apply rollback)

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
