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
`/api/node` (`link_local_lan` / `link_local_mesh`). Nodes also have an
identity-derived ULA (`blake3(b"mjolnir/mesh/ula/v0")` `/48` +
`blake3(node_id)` IID) on `br-mesh` beside `10.254`; it is not an iroh
QUIC candidate.
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

Lightning Admin also hosts owner-drawn `canvas-*` overlays whose wells
show allowlisted host stdout. The first source is macOS `/sbin/ifconfig`
with no extra args, a 5 second timeout, and a 65536 byte capture cap.
`scan_link_local` remains the neighbor list; the
well is a raw transcript, not a second parser. One canvas now; source
swap (`add-view-swap`) and a crystal-grid of many overlays
(`add-crystal-grid`) are later. Living spec:
`openspec/specs/admin-stdout-views/spec.md`.

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

Mini-apps (`add-mini-app-contract`, bead `mjolnir-mesh-ncy.1`): a
published service is an app when TXT carries `app=v1`. Insertion is a
tap-to-load sandboxed iframe on the **app's own origin**; app script,
markup and style never enter a hello.mesh document. hello.mesh presents
them in an Apps shelf above Services (`add-app-shelf`, `ncy.3`); the
shelf consumes that living insertion. The visitor's
rung-1 key lives in IndexedDB of hello origins (`http://hello.mesh`
and every LAN gateway `http://10.42.<x>.1`), so `canEmbed` refuses
every key-bearing origin structurally: IP-literal hosts, reserved
names (`hello.mesh`, `id.mesh`, and any future name that serves the
hello page), and the current page origin. Manifests are fetched
node-side (`GET /api/apps`); the visitor's browser contacts no app
host until they open one. Identity over the bridge
(`add-embedded-assert`, bead `mjolnir-mesh-ncy.4`) is the same
`mjolnir-identity-assert` v1 token as `/assert`, carried as
`identity.request` (window message plus one MessagePort) and
`identity.response` (that port only). Audience is the card's entry
origin. Consent is drawn by hello.mesh outside the iframe. One
pending request, a 60 s host deadline, cooldown keyed by service
name. Framed hello.mesh (`window.top !== window`, including a hello
origin) fails closed before identity load; application HTML carries
`frame-ancestors 'self'`, not the captive portal.

Gossip is not authority for admin control (`add-control-record-integrity`,
bead `mjolnir-mesh-ai0.9`). A node uses an owner-signed record only after
it verifies the signature, domain, house, epoch, and canonical bytes
itself. Those bytes are the mesh-admin-capabilities canonical-CBOR
profile, domain `lightning-admin/record/<type>` version 1. Node-vouched
facts never satisfy an owner-signed check. Freshness advances only from
owner-signed authority-state records. A forked authority head is refused
until a higher-epoch successor names one branch. This capability does
not write a new head. Living spec:
`openspec/specs/admin-record-integrity/spec.md`.

Wi-Fi internet uplink (`add-wifi-internet-uplink`, bead
`mjolnir-mesh-bf7.2`): one authorized node joins a selected upstream
SSID as a station and feeds the mesh. Credentials bind to that candidate
(node id, radio id, SSID; a different BSSID is a new candidate). The
passphrase lives in a root-only file outside the four revisioned UCI
files. A DHCP lease is not downstream internet. The mesh default-route
export is not a second default beside an existing house-gateway export
unless that change is in the same revision-checked plan. Failure
restores the previous committed plan. The station interface is not
bridged into the client LAN. Living spec:
`openspec/specs/wifi-internet-uplink/spec.md`.

Wired node profiles (`add-wired-node-profiles`, bead `mjolnir-mesh-ai0.6`):
each used port is WAN, node-link, client, or disabled, and each used
radio is mesh, client-ap, station, or disabled. The default stays
`ethernet-gateway-satellites`. `wired-core` needs a qualified port map.
A node-link stays off `br-mesh` and off resident DHCP. meshd feeds
`mjolnir.node_link` into the babel renderer as `type wired` and does
not drop a house-gateway `0.0.0.0/0` export. Living spec:
`openspec/specs/node-connection-profiles/spec.md`.

Household owner claim (`add-household-owner-claim`, bead
`mjolnir-mesh-bf7.1`): one owner binds only when device proof, a card
HMAC, and a fresh 120-second local button window match the same claim.
The node stores `k`, not the card string. Recovery needs an enrolled
credential plus a new window, advances the ownership epoch and the
authority epoch, and does not elect a fork winner. The setup button
toggles WAN SSH when there is no verifier, or when the node is already
owned and no recovery credential has verified. A ceremony press does
not. hello.mesh relays and never sees the secret. Living spec:
`openspec/specs/household-owner-claim/spec.md`.

## Pointers

- Overlay addressing and radio backhaul: `docs/network-coordination/`
- Allocation-free IPv6 parallel plane (SLAAC / identity ULA / app overlay):
  `docs/network-coordination/ipv6-parallel-plane.md` (epic `mjolnir-mesh-v6`;
  does not reverse the `bsa` spine rejection)
- Front desk / hello.mesh: `crates/mjolnir-hello/`, `docs/products/hello.mesh/`
- Owner-signed admin records: `openspec/specs/admin-record-integrity/spec.md`
- Station internet uplink: `openspec/specs/wifi-internet-uplink/spec.md`
- Wired node profiles: `openspec/specs/node-connection-profiles/spec.md`
- Household owner claim: `openspec/specs/household-owner-claim/spec.md`
- Captive portal (offer IdentiKey, or pass through): `openspec/specs/captive-portal/spec.md`
- Client network name (SSID ≠ guild): `openspec/specs/client-network-name/spec.md`
- Lightning settings store: `openspec/specs/mjolnir-settings/spec.md`
- Lightning Admin stdout-view canvases (owner-drawn `canvas-*` overlays,
  allowlisted host argv, first source macOS `/sbin/ifconfig`):
  `openspec/specs/admin-stdout-views/spec.md`
- Coverage survey (directory last-known coords, phone stamp, `/mesh` viz,
  fixture walk paint unknown/thin/covered + completeness):
  `openspec/specs/mesh-coverage/spec.md`
- WAN-LAN admin (WPS-armed prefix-boxed nft WAN SSH):
  `openspec/specs/wan-lan-admin/spec.md`
- Link-local first-contact SSH (`br-lan` / `br-mesh` `fe80`; overlay
  `10.254` stays mesh-wide): `openspec/specs/link-local-mgmt/spec.md`
- Identity-derived ULA on `br-mesh` beside `10.254` (not an iroh
  candidate): `openspec/specs/identity-derived-ula/spec.md`
- Mini-apps (TXT `app=v1`, node-side manifest fetch, Apps shelf,
  sandboxed cross-origin card; key origin ≠ app origin; identity
  over the bridge via one-shot MessagePort):
  `openspec/specs/mesh-mini-apps/spec.md`
- Laptop-local OpenWrt sim lab (named libvirt q35 domains, mgmt
  `10.99.0.0/24` not overlapping host/Docker bridges, vwifi air,
  virtio NAT; household double-NAT: node-a WAN on ISP-CPE LAN with
  `gateway=auto`, node-b WAN behind house-nat without default export,
  overlay `10.254` on 802.11s not WAN hole-punch; not Filogic; sim
  is not metal roam proof):
  `openspec/specs/mesh-sim-lab/spec.md`,
  contract `deploy/sim/README.md`
