# Add client roam continuity

> **ACTIVE BUILD**

**Rigor:** architecture. Activated by Duke in chat, 2026-11-13 ("spec it out
and launch some subagents for it").

## Why

A phone on a call that moves between two nodes can go silent for seconds. The
keep-your-address path (`sz9` mobility `/32` + proxy ARP) only holds while the
phone never asks DHCP again. Field and sandbox evidence, 2026-11-13:

- **Re-DHCP renumbers the phone.** A node asked to confirm another node's
  address answers NAK at once (`dhcp-authoritative`, own range only), then
  offers an address in its own `/24` after a 3 s ping check. Reproduced
  against dnsmasq 2.93.
- **Lease gossip never starts.** The dnsmasq hook
  `/usr/libexec/mjolnir-dhcp-event` runs inside dnsmasq's `ujail`. It is
  mounted read-only, runs as `dnsmasq`, and its spool `/tmp/mjolnir/` is not
  in the jail. No event is ever written. Both live nodes hold leases and an
  empty `mjolnir-roam.conf`. OpenWrt also *sources* the hook, so its
  `set -e` and `exit` cut the stock hotplug handler short.
- **Reservations would not load anyway.** `dhcp-host` lines sit in a
  `conf-dir` file. dnsmasq does not re-read `conf-dir` on SIGHUP. And a
  reservation outside the interface's own subnet is ignored without a
  `shared-network` declaration.
- **No roam assistance.** 802.11k/v is off. Phones get no neighbour list and
  no transition hints. hostapd on the fleet supports both over ubus.
- **Proxy ARP answers late.** `proxy_delay` is 80 (up to 0.8 s).

`mjolnir-mesh-0wgr.5` recorded the DHCP failure in the sim lab on 2026-09-25
and `wvg.4` stayed open. This change closes it.

## What

1. **Leaderless DHCP continuity.** Every node keeps vending only its own
   `/24`. Every node also confirms any address in the gossiped lease book,
   whichever node vended it. No leader, no shared pool, no client L2 bridge.
2. **Lease gossip that works under the jail.** meshd reads dnsmasq's lease
   file. The hook script is removed.
3. **Roam assistance.** Nodes gossip their 802.11k neighbour report and
   install the peers' reports in hostapd, with 802.11v BSS transition on.
4. **Prompt proxy ARP.** `proxy_delay=0` on the client bridge.

Not in this change: a shared gateway MAC (see design.md, bead filed). That
needs a decision on shared copper before it touches a live bridge.

## Impact

- Capability: ADDED `client-roam`.
- Modified: `mesh-sim-lab` DHCP CRDT e2e now checks the hosts file.
- Code: `crates/mjolnir-mesh/src/crdt/lease.rs`, new `roam_assist.rs`,
  `mjolnir-meshd.rs` loops, `deploy/openwrt` (hook removed).
- Wire: one appended `GossipMessage` variant. Old nodes fail to decode it and drop that one message.
- Rollout: fleet update through `update-fleet.sh`. dnsmasq restarts once per
  node when the static roam config first lands.
- Beads: `wvg.4` (DHCP), `0pv` (physical walk) stay open until a metal walk.

## User journey & surfaces

No new UI because this is dataplane behaviour on the client SSID. The
surface is a phone on `Lightning Mesh`: a call continues while the person
walks between nodes, and the phone's address does not change. Operators see
it in `logread` (`dhcp roam:` and `roam assist:` lines) and in
`deploy/sim/e2e/run.sh dhcp-crdt`.
