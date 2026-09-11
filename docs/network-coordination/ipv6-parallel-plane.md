# Allocation-free IPv6 as a parallel plane

**Status 2026-09-11:** AGREED direction (epic `mjolnir-mesh-v6`). Not shipped.
**Does not reverse** [`ipv6-addressing-decision.md`](ipv6-addressing-decision.md)
(`bsa`): there is still no IPv6 *spine*, no v4-via-v6, no AAAA-only `.mesh`,
and iroh node-ids remain the non-scarce identity layer.

This note is the additive plane: addresses nobody has to dole out, plus what
an **app running our code** gets that a dumb phone on the SSID does not.

---

## What's an RA? (and what SLAAC is)

**RA** = Router Advertisement. An ICMPv6 packet the router multicasts on the
link, every few seconds and whenever a host asks.

DHCP is a clerk with a book: the phone asks “may I have an address?”, the
server looks up the MAC, writes a lease, hands back `10.42.5.23`. Someone
had to remember that number so nobody else got it. That is why we built a
subnet-claim CRDT and sketched a `/devices/{mac}` lease lane.

An RA is a loudspeaker in the room, not a clerk:

> This link’s prefix is `fdxx:…:/64`. I am a router. MTU is 1500. DNS is
> here. Captive portal is at `http://hello.mesh/api/captive-portal`.

The phone hears that, **picks the rest of the address itself**, pings once
to check nobody else took the same host bits (DAD), and is on the network.
Nobody wrote it down. That process is **SLAAC** — Stateless Address
Autoconfiguration (RFC 4862). Stateless means there is no lease table.

The RA is also how IPv6-first phones (iOS especially) learn the captive
portal **before they have an IPv4 lease**. That is the same failure mode
link-local already saved us from, and why CAPPORT (RFC 8910) belongs on
the RA (`aj1` / `v6.3`), not only on DHCP option 114.

---

## Does SLAAC work well?

Yes. It is how IPv6 has worked on home LANs, phones, and ISP CPEs for
twenty years. OpenWrt’s `odhcpd` already speaks it; we have just been
ignoring it (and filtering the stock random ULA prefixes out of babel).

What it is good at:

- Host assignment on **one L2** (one node’s client AP) with no server,
  no CRDT, no MAC reservation.
- Coming up when DHCP is late, broken, or absent — the original LL win.
- Prefix changes: the host deprecates the old address and SLAACs a new one.

What it is not:

- It does not route. A prefix that babel does not export is local to that
  AP. Same as today: we still need babel (and `mjolnir0` across sites) for
  off-link IP.
- It does not give you an identity. Default SLAAC host bits are a MAC
  (EUI-64) or a privacy temporary (RFC 4941). Phones *should* use privacy
  addresses; IP is not who they are.
- It does not replace IPv4 for v4-only clients. RFC 6724 still prefers
  IPv4 over ULA when both exist, so dual-published names will keep using
  `10.42`.
- It does not survive a roam with the *same* address unless the prefix is
  the same on the next AP (or the device is not using SLAAC at all — see
  apps below).

Honest caveats: some ancient IoT is v4-only; DAD needs link multicast
(fine on one AP, do not flood it mesh-wide); source selection across
several prefixes is an OS policy, not ours.

---

## What we were reinventing

A large part of the IPv4 control plane is “someone must pick a number and
everyone else must not pick the same one.” For IPv6 host bits, the IETF
already shipped that as SLAAC. For IPv6 *node* addresses, RFC 4193 ULA
already says “hash something unique into a 40-bit global id.” We hashed
into 16 bits of `10.254` instead and then built `pt9` to recover from
collisions.

**Rule: if a thing has an identity (iroh node-id, later IdentiKey), its
ULA is a hash of that identity. Never allocate it.**

Routers, Admin, VMs, future client apps: derive, assign, done. Phones
that do not run our code: SLAAC from the node’s RA’d `/64` (still no
lease book). IPv4 DHCP + `/subnets/` CRDT stay for the v4 access edge.

### What SLAAC + identity-ULA replace vs keep

| Ad-hoc we brewed | v6 fate |
|---|---|
| IPv6 half of `LeaseEntry` / `/devices/{mac}` | **Do not wire.** SLAAC is the v6 lease. |
| DHCPv6 | **Do not run.** RA + SLAAC. |
| `pt9` collision claims, but for ULA | **Do not need.** 64 bits of `blake3(node_id)` in the host; birthday cliff is gone. |
| Stock OpenWrt `network.globals.ula_prefix` (random `/48` per box) | **Ignore / keep filtered.** Not identity-derived; live pair already has two different `/48`s. |
| IPv4 DHCP + `/subnets/` CRDT | **Keep.** Dumb phones, v4-only stacks. |
| `10.254` derived IPv4 + `pt9` | **Keep** as the iroh underlay until ULA-as-QUIC-candidate is re-measured. Parallel, not a swap. |
| Guest roam `/32`s (`sz9`) | **Keep for IPv4.** v6 phones re-SLAAC; app ULAs do not roam (they never change). |
| Gossip address book (`0yb`) | **Keep.** That maps node-id → *iroh dial candidates*, not “who has this IP.” |
| `.mesh` DNS | **Keep.** Names stay the UX; add AAAA once ULA exists (`v6.5`). |

---

## Two populations

```
dumb phone on the SSID          app running our code
──────────────────────          ────────────────────
DHCP 10.42.x  (keep)            no DHCP
SLAAC from RA /64               self-assign ULA = hash(node_id)
IP roam = new address           address never changes
traffic: AP → babel → …         iroh dials the peer node-id
identity: none (MAC at best)    iroh node-id IS the identity
```

The phone is why IPv4 CRDT exists. The app is why Tailscale feels easy.
We need both; they must not share an allocator.

---

## What an app running our code does better

Lightning Admin today, a future mesh client, a Mjolnir VM, anything that
links `mjolnir-mesh` and holds an Ed25519 node-id.

**Self-assign a ULA.** Same function as `backhaul_addr()` in
`tun/link.rs`, with enough bits. Put it on a local TUN. No RA required
(the app is not waiting for a router to describe the prefix). No DHCP.
No CRDT. Reboot with the same secret → same address.

**iroh-reachable by construction.** The ULA is a convenience for
IP-shaped tools (`ping`, `ssh`, a browser). The thing that actually
finds the peer is the node-id. Local vs nonlocal is iroh’s path
selection (LAN DIRECT, hole-punch, relay) — not babel, not “which
`/24` is this MAC in.”

**No hop through a mesh router to reach another iroh endpoint.** Two
apps on the same island dial DIRECT. Two apps at different sites dial
over iroh the same way the routers already do. The phone path is
always `phone → AP → babel [→ mjolnir0] → dest`. The app path does not
borrow a lease, does not wait for a `/32` to follow it, and does not
need the destination to live in a claimed subnet.

**Roam is free.** The address is a function of the key, not of which AP
radio it heard. `sz9` exists because a phone kept `10.42.5.23` after it
walked to another node’s `/24`. An app never did that.

**Reuse the per-peer TUN primitive, not the router’s N-tunnel mesh.**
`tun/encap.rs` (one TUN ↔ one iroh connection) is the app datapath.
`mjolnir0` stays the router multiplexer for *dumb-client* IP across
sites. `lan_tunnels=1` (N babel interfaces on the router) stays off.

**Still speak `.mesh`.** MagicDNS-style: the app *is* the stub
(`v6.5`). It can resolve `hello.mesh` / `wiki.mesh` without a DHCP DNS
server on `10.42.x.1`. AAAA once the derived ULA exists; never NXDOMAIN
on AAAA for a name that has A.

What the app does *not* get to skip: membership, the radio, or talking
to a printer that is just a phone on someone else’s `/24`. Those still
go through the access edge.

---

## Stones (epic `mjolnir-mesh-v6`)

1. Link-local as a management address (`v6.1` / `add-link-local-mgmt`) —
   first contact, no overlay. `directory.json` `node.link_local_lan` /
   `link_local_mesh` are kernel `fe80` on `br-lan` / `br-mesh`, not
   `mjolnir0`'s babel TUN LL. Operator adds `%iface` on their NIC.
2. Identity-derived ULA on nodes (`v6.2` / `add-identity-derived-ula`) —
   `ula_addr`: `/48` = `fd` + 40 bits of `blake3(b"mjolnir/mesh/ula/v0")`,
   subnet 0, IID = 64 bits of `blake3(node_id)`. Assigned `/64` on
   `br-mesh` only (`nodad` / reconcile). Not on `mjolnir0`. Not an iroh
   candidate. Babel `in`/`out ip fc00::/7 deny`.
3. RA that `/64` on the client AP + CAPPORT on the RA (`v6.3`).
4. Optional babel IPv6 FIB (`v6.4`) — never v4-via-v6.
5. `.mesh` AAAA + app-side resolver (`v6.5`).
6. App TUN + own iroh (`v6.6`) — this section.

Ready now: `v6.1` and `v6.2` in parallel.
