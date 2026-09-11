# Architecture

Why Lightning Mesh is shaped this way. Amend; do not delete prior notes.
Product motivation lives in `docs/vision/`. Coordination decisions live in
`docs/network-coordination/`. What is built lives in `openspec/specs/`.

## Shape

The L3 overlay (iroh + babeld + CRDT) is the product. The radio is plumbing.
Nodes are symmetric and non-authoritative. Each node owns a routed `/24`;
client L2 is never bridged across nodes.

Management is the overlay: reach nodes at derived `10.254.x` over SSH
once you are on the mesh. First contact on the LAN/SSID is this node's
link-local (`fe80` on `br-lan` / `br-mesh`), published on hello
`/api/node` (`link_local_lan` / `link_local_mesh`).
No mDNS for mesh-wide discovery — gossip/CRDT is the address book.

Disruptive node changes go through `mjolnir-apply` (snapshot → apply →
health gate → rollback), never a live SSH mutation.

The client AP SSID is a public **network name** (`client-network-name`),
not a guild. Factory default is ⚡ open. Radio apply does not mint
keyspace membership. Guild key material lives in identikey-core.

Lightning Admin is a writer of that radio apply (`apply-network-name.sh`
→ `mjolnir-apply` `RUN_WIRELESS=1`, no meshd binary). From a house WAN
LAN the overlay is unreachable; `LIGHTNING_FLEET_SSH` names WPS-armed
WAN `root@` addrs. Discovery stays a link-local scan and does not
SSH-mutate UCI. `update-fleet.sh` remains the full binary+radio rollout.
WPS arms a prefix-boxed nft WAN SSH window; not UCI persist; overlay
remains the management plane.

`/etc/config/mjolnir` (`radio`, `wan_admin`) is the live operator store.
`mjolnir-apply` is the only projector of those knobs onto UCI `wireless`.
A staged `wireless.env` writes the store and is then wiped. LuCI-edited
`wireless` is not canonical.

Coverage survey (`add-coverage-survey`, bead `mjolnir-mesh-6hn`): last-known
WGS84 lives on the gossiped directory projection, not in `radio.json` v1
and not in UCI. Join to radio is `backhaul_addr`. A DreamBall is a signed
snapshot plus render recipe, not the live store; export is an explicit
operator act. v1 surveyor is a phone on the client SSID: associated AP
plus nearby radio strength, operator picks a node, enters GPS. Stamp
ingress is a signed claim to hello on the node's LAN gateway, spooled to
meshd; hello verifies only; the CRDT is the authority. Routers are Wi-Fi
and gain no NFC, BLE, or ESP-NOW. BLE/ESP-NOW is handheld-to-handheld
only. Compass magnetometer exists; compass GPS is later. World-model
capture is the AI Camera (cameras + Ethernet 3D lidar), not the LilyGo
T-RGB. Visualizer home is web3d-space `/mesh`; the stamp UI is the
hello.mesh Routers panel on the LAN gateway (pick nearby + GPS) posting
to `/api/coordinate-stamp`. Last-known is LWW by `stamped_at` + stamper;
the viewer ages stale stamps. Last-known is now projected from a
CoordinateBook (i32 e7, LWW unix, gossip CoordinateAnnounce). Do not
confuse this with `e21` “north-star” roaming. Fixture coverage walks
paint unknown / thin / covered cells on web3d-space `/mesh` with an
on-screen completeness score (`add-coverage-sweep`). First slice is
recorded-walk replay on the four-router fixture, not live fleet RF
heatmaps.

## Pointers

- Overlay addressing and radio backhaul: `docs/network-coordination/`
- Allocation-free IPv6 parallel plane (SLAAC / identity ULA / app overlay):
  `docs/network-coordination/ipv6-parallel-plane.md` (epic `mjolnir-mesh-v6`;
  does not reverse the `bsa` spine rejection)
- Front desk / hello.mesh: `crates/mjolnir-hello/`, `docs/products/hello.mesh/`
- Captive portal (offer IdentiKey, or pass through): `openspec/specs/captive-portal/spec.md`
- Client network name (SSID ≠ guild): `openspec/specs/client-network-name/spec.md`
- Lightning settings store: `openspec/specs/mjolnir-settings/spec.md`
- Coverage survey (directory last-known coords, phone stamp, `/mesh` viz,
  fixture walk paint unknown/thin/covered + completeness):
  `openspec/specs/mesh-coverage/spec.md`
- WAN-LAN admin (WPS-armed prefix-boxed nft WAN SSH):
  `openspec/specs/wan-lan-admin/spec.md`
- Link-local first-contact SSH (`br-lan` / `br-mesh` `fe80`; overlay
  `10.254` stays mesh-wide): `openspec/specs/link-local-mgmt/spec.md`
