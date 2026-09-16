# add-zero-touch-join

> **ACTIVE BUILD**

Bead epic `mjolnir-mesh-fyby`. Human activated 2026-09-16 ("activate all").
Steer 2026-09-16 recorded on `fyby.1` (all three recommended forks).
Sol consult 2026-09-15: caution.

## Why

A factory Lightning Mesh router still needs a laptop and a hand-edited
`list peer` on every box (`ap3000-outdoor-b`, 2026-09-15). The operator
wants plug-in, wait, find nearby Lightning Mesh with no peer file. iroh
already authenticates iroh traffic; it does not authenticate 802.11s or
babel. Treating association as membership would let a stranger in RF
range inject routes, poison claims, and reach overlay SSH. The product
move is automatic **discovery**, not automatic **trust**.

## What

- Name the capability `identity-peering`.
- Association on `mjolnir-mesh` is not authority. Unknown identities
  quarantine: handshake + enrollment lane only.
- Signed capability beacon (`e5u`) is a locator, not a ticket. Untrusted
  compatible meshes get an enrollment **offer**; `met` QR remains the
  physical-presence path.
- Control plane (babel routes, CRDT writes, subnet claims, overlay SSH)
  waits on three gates: quarantine (`fyby.2`), authenticated babel
  neighbor/session/freshness (`661`), identity-authorized CRDT writes
  (`fyby.3`). Until then trusted-only / inventory bootstrap is honest.
- Trusted-fleet installs still get a **full** peer set from
  `fleet-nodes.conf` (`m4a`). RF neighbors are never ingested into
  `list peer`.
- Incompatible or untrusted peers degrade to isolated L3/NAT without
  overlay management or unrestricted relay.

## Impact

- Capabilities: ADDED `identity-peering`
- ADRs: will amend `ARCHITECTURE.md` (discoverable ≠ trusted; radio is
  a hostile underlay until the three gates)

## User journey & surfaces

A person powers a factory node in RF range of an existing island. The
node finds Lightning Mesh from the capability beacon and shows as heard
(hello / LED / log). Empty: no matching beacon, it stays a lone island
with its own `10.42`. Failed: it associated but stays quarantined —
babel has no adjacency, overlay SSH from the stranger fails, claims are
ignored. Off: today's hand `list peer` on a trusted inventory install
(`install-node.sh` + `fleet-nodes.conf`) still bootstraps gossip without
waiting for this capability.

Enrollment: an existing member accepts the beacon offer, or scans the
new node's QR (`met`). After a capability grant, the new id is gossiped
and may speak on babel/CRDT.

No new UI because the surfaces are the existing 802.11s backhaul, hello.mesh
directory, dropbear on overlay/LAN, and `install-node.sh`. QR is the
already-designed `met` path.

## Out of scope

- Local-EVM transit settlement (`mjolnir-mesh-9u5y`, parked)
- VXLAN / EVPN-lite client islands (`3kd`) and trusted shared-L2 (`190`)
- User IdentiKey (`rp9`)
- Auto-ingest of 802.11s neighbors into `list peer` (forbidden by steer)
- Changing the live fleet `MESH_KEY` / open client SSID in this change
- Factory flash/sysupgrade (still OpenWrt + `install-node.sh`)
