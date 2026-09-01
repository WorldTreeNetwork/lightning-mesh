---
name: mesh-errand
description: >
  Leave known-good Wi-Fi, do one errand on a mesh node via the client SSID,
  restore the working uplink, then interpret the capture. The laptop has one
  radio; joining Lightning Mesh / ⚡ drops Pirate Radio / Origami. Run
  deploy/openwrt/mesh-errand.sh with ERRAND set to the on-box work. Use when
  the user wants logs, DHCP, ping, or SSH on a node that has no internet, a
  phone spinner on the mesh SSID, "can't join from the laptop", or
  /mesh-errand. Formerly /probe-client-ssid.
---

# Mesh errand

Implementation: `deploy/openwrt/mesh-errand.sh`. Do not reimplement the
join/restore loop.

**errand** = the work that happens on the box (or on the mesh SSID) before
coming home. The agent turns the user's ask into a shell snippet, runs it
over there, then processes the capture on known-good internet.

## Do

1. Confirm the laptop is on the working uplink (Pirate Radio, Origami, …).
2. Compile the errand. Env inside `ERRAND`: `ADDR` `GW` `DNS` `IFACE` `SSID`
   `HOME_CON`. Prefer `root@$GW` over overlay addrs (overlay is often down
   from the client SSID).
3. Run from repo root (timeout ≥ 90s if the errand SSHs):

```bash
ERRAND='ssh -o BatchMode=yes -o ConnectTimeout=5 root@$GW "uci get mjolnir.meshd.name; ip -4 addr; logread | tail -30"' \
  deploy/openwrt/mesh-errand.sh
```

Empty `ERRAND` = DHCP + ping `$GW` only.

Overrides: `SSID` (NetworkManager connection **id**), `HOME_CON`, `WAIT`
(seconds, default 25). If the fleet beacons `⚡` but the NM profile is still
`Lightning Mesh`, pass that id as `SSID`.

4. After EXIT, confirm `nmcli` is back on `HOME_CON` before claiming anything.

## Read the capture

| Line | Meaning |
|---|---|
| `JOIN_FAILED` | Did not associate. |
| `DHCP_FAILED` | Associated, no IPv4 lease. |
| `NO_GATEWAY` | Lease without a router option. |
| `GATEWAY_BLACKHOLE <ip>` | Lease ok, ping to `.1` fails (dual `192.168.1.1/24` on `br-lan`). |
| `GATEWAY_OK <ip>` | Unicast to the node works. |
| `ERRAND_EXIT=N` | On-box snippet finished with that status (restore already ran). |

## Do not

- Leave the radio on the test SSID.
- `ip route replace default via 10.42.x.1` while the chat uses the working uplink.
- Open a long interactive session on the test SSID.
