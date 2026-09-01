---
name: probe-client-ssid
description: >
  Probe a mesh/client Wi-Fi SSID that may have no internet without stranding
  the agent chat. The laptop has one Wi-Fi radio; joining the test SSID drops
  Pirate Radio / Origami. Run deploy/openwrt/probe-client-ssid.sh (join, print
  DHCP + ping to 10.42.x.1, always restore). Use when the user mentions a
  phone spinner on Lightning Mesh, no IP / DHCP on the client SSID, "can't
  join the mesh from the laptop", no internet on Lightning Mesh, or
  /probe-client-ssid.
---

# Probe client SSID (restore working Wi-Fi)

The implementation is `deploy/openwrt/probe-client-ssid.sh`. Do not reimplement
the join/restore loop in a one-off shell.

## When

The network under test (client SSID, usually `Lightning Mesh` or `⚡`) has no
internet, or a phone sits on Connecting…. You need DHCP/gateway evidence
without leaving the laptop on that SSID.

## Do

1. Confirm the laptop is on the **working** uplink (Pirate Radio, Origami, …).
2. Run from the repo root (timeout ≥ 60s — NM join + restore):

```bash
deploy/openwrt/probe-client-ssid.sh
```

Overrides: `SSID` (NM connection name or SSID), `HOME_CON` (restore target;
default = current Wi-Fi), `WAIT` (seconds, default 25).

```bash
SSID='Lightning Mesh' HOME_CON='IdentiKey Pirate Radio' WAIT=30 \
  deploy/openwrt/probe-client-ssid.sh
```

3. If the fleet beacons `⚡` and the NM profile is still named `Lightning Mesh`,
   pass `SSID` as the **NetworkManager connection id**, not the UTF-8 SSID.
4. After EXIT, confirm `nmcli` is back on `HOME_CON` before claiming anything.

## Read the last line

| Line | Meaning |
|---|---|
| `JOIN_FAILED` | Did not associate (wrong security, SSID down, bad NM profile). |
| `DHCP_FAILED` | Associated, no IPv4 lease. Phone spinner can be DHCP. |
| `NO_GATEWAY` | Lease without a router option. |
| `GATEWAY_BLACKHOLE <ip>` | Lease ok, ping to `10.42.x.1` fails. Dual `192.168.1.1/24` on `br-lan` is the known shape. Phone spinner is often this, not missing DHCP. |
| `GATEWAY_OK <ip>` | Unicast to the node works; internet is a separate NAT/babel question. |

## Do not

- Leave the radio on the test SSID.
- `ip route replace default via 10.42.x.1` while the chat uses the working uplink.
- Open a long interactive `nmcli` session on the test SSID.
