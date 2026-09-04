# Architecture

Why Lightning Mesh is shaped this way. Amend; do not delete prior notes.
Product motivation lives in `docs/vision/`. Coordination decisions live in
`docs/network-coordination/`. What is built lives in `openspec/specs/`.

## Shape

The L3 overlay (iroh + babeld + CRDT) is the product. The radio is plumbing.
Nodes are symmetric and non-authoritative. Each node owns a routed `/24`;
client L2 is never bridged across nodes.

Management is the overlay: reach nodes at derived `10.254.x` over SSH.
No mDNS for mesh-wide discovery — gossip/CRDT is the address book.

Disruptive node changes go through `mjolnir-apply` (snapshot → apply →
health gate → rollback), never a live SSH mutation.

The client AP SSID is a public **network name** (`client-network-name`),
not a guild. Factory default is ⚡ open. Radio apply does not mint
keyspace membership. Guild key material lives in identikey-core.

Lightning Admin is a writer of that radio apply (`update-fleet.sh
--wireless` → `mjolnir-apply`). Discovery stays a link-local scan and
does not SSH-mutate UCI.

## Pointers

- Overlay addressing and radio backhaul: `docs/network-coordination/`
- Front desk / hello.mesh: `crates/mjolnir-hello/`, `docs/products/hello.mesh/`
- Captive portal (offer IdentiKey, or pass through): `openspec/specs/captive-portal/spec.md`
- Client network name (SSID ≠ guild): `openspec/specs/client-network-name/spec.md`
