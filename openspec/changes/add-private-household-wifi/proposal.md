# add-private-household-wifi

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

The house default is private resident Wi-Fi, with an optional guest
network that is isolated across nodes. The fleet on the air today is
still the open client SSID. Recording that default must not encrypt
the radios by itself.

## What

- Build a resident-Wi-Fi plan and an optional guest plan as a
  network-change transaction.
- Keep the passphrase out of the four revisioned UCI files, receipts,
  and hello.mesh.
- Refuse to label a guest plan isolated until it denies guest-to-resident
  and cross-node guest reachability on IPv4 and on IPv6 when IPv6 is
  enabled, without bridging `br-lan`.
- Leave the deployed open SSID in place until an owner commits this plan.

## Impact

- Capabilities: ADDED private-household-wifi.
- ADRs: none.
- Work node: mjolnir-mesh-ai0.7. Depends on folded wired node profiles.
  Activation inherited from Duke's `activate ai0`. No live radio writes.
- Paid transit stays parked. Biscuit issuance stays `mjolnir-mesh-ai0.1`.

## User journey & surfaces

No new UI because the owner still applies network changes through the
existing `mjolnir-apply` transaction. The future Manage network surface
is not this change. Until that transaction commits, phones keep seeing
the open client SSID. After it commits, resident Wi-Fi asks for the
house passphrase, and an enabled guest SSID cannot open a resident
service on another node unless the plan names that service as an
exception. A plan that cannot state the isolation is refused and the
previous revision stays.

## Out of scope

- Live radio writes and which radio serves clients (`mjolnir-mesh-lpv`).
- Health-probe implementation (`mjolnir-mesh-2mgv`).
- Paid transit.
- Biscuit engine and grant profiles (`mjolnir-mesh-ai0.1`).
- CAPPORT or welcome-page behavior.
- Bridging client L2 segments across nodes.
- A passphrase field on hello.mesh or in Lightning Admin in this change.
