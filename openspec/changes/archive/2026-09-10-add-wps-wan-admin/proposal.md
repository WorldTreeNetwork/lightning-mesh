# add-wps-wan-admin

> **ACTIVE BUILD**

Bead `mjolnir-mesh-m0d`. Human activated 2026-09-03.

## Why

A Lightning node on someone else's CPE (house NETGEAR, Origami, …) is
reachable by ICMP on its WAN lease, but OpenWrt `wan` input is REJECT, so
SSH from that LAN is closed. Stock WPS only starts hostapd `wps_pbc`, which
is a no-op on the open client AP. Operators standing next to the box need a
physical-presence gate to administer it from the WAN LAN without leaving
TCP/22 open to the internet.

## What

- Name the capability `wan-lan-admin`.
- WPS short-press toggles a 15-minute nft window: TCP/22 from the WAN
  **connected prefixes only** (IPv4 subnet + IPv6 global prefixes on the WAN
  iface). Second press or reboot closes it. No UCI persist.
- Hook `/etc/rc.wps/00-mjolnir-wan-admin` so it runs before stock
  `40-wps_ap` / `50-wps_sta` and consumes the press (OpenWrt 25.12
  `/etc/rc.button/wps` breaks on first success).
- Merge operator pubkeys into `/etc/dropbear/authorized_keys` on apply
  (opening the port without a key is still `Permission denied`).
- Blink the WPS LED while armed. Do not expose LuCI/hello on WAN.

## Impact

- Capabilities: ADDED `wan-lan-admin`
- ADRs: none

## User journey & surfaces

An operator on the same LAN as the node's WAN (e.g. `192.168.0.18` talking
to `192.168.0.15`) presses the WPS button. The WPS LED blinks. `ssh root@<wan
lease>` works for 15 minutes with an authorized pubkey. Empty: no WAN lease,
press logs and does not open `0.0.0.0/0`. Failed: second press or reboot
closes the window; `fw4` reload also drops it. Off: button never pressed,
WAN `:22` stays refused.

No new UI because the surface is the physical WPS button plus existing
dropbear.

## Out of scope

- Disabling `dropbear.PasswordAuth` (already on this fleet; window is
  prefix- and time-boxed)
- Always-on UCI `Allow-SSH-WAN` (AP3000 leftover; anti-pattern)
- Real Wi-Fi WPS-PBC / STA enroll (`m50`)
- Reset-button failsafe/factory
- `hello.mesh` / LuCI on WAN
- Client-SSID DHCPv4 (`dsd`)
