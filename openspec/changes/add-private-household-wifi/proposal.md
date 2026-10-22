# add-private-household-wifi

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

The house has one client SSID. A second "guest" network hid the service
mesh and did not create bandwidth. Residents need their flows to go
first when the link is busy. Plugged-in services must stay reachable
by a `.mesh` name.

## What

- One client SSID. No visitor SSID, no guest prefix, no guest isolation
  gate.
- Three classes on that SSID: a published service, a member, and an
  unidentified client.
- When the link is busy, member flows are scheduled ahead of
  unidentified flows. No rate cap.
- A published `.mesh` name stays reachable from every client.
  Clients do not gain a route to every house address. The management
  plane stays closed.
- Creating an identity is not required for internet or for published
  names. SSID association is not membership.
- An uncommitted plan does not change the live SSID.

## Impact

- Capabilities: ADDED private-household-wifi.
- ADRs: none.
- Work node: `mjolnir-mesh-ai0.7`. Activation inherited from Duke's
  `activate ai0`. No live radio writes.
- Guest follow-ups `mjolnir-mesh-mzgw`, `mjolnir-mesh-fyby.4`, and
  `mjolnir-mesh-nx1k` are superseded.
- The priority mark waits on `mjolnir-mesh-ai0.1`. This change states
  the rule and does not issue biscuits.
- Paid transit stays parked.

## User journey & surfaces

No new UI because the owner still applies network changes through the
existing `mjolnir-apply` transaction. The future Manage network surface
is not this change. Until a later owner-committed plan says otherwise,
phones keep seeing the one open client SSID. A member who has signed
in is scheduled ahead of an unidentified client when the link is busy.
Both can open a published `.mesh` name. Neither can open the management
plane.

## Out of scope

- A second SSID, a guest prefix, or a hard rate cap.
- Live radio writes (`mjolnir-mesh-lpv`).
- Health-probe implementation (`mjolnir-mesh-2mgv`).
- Paid transit.
- Biscuit issuance (`mjolnir-mesh-ai0.1`).
- Installing an SSH key for a guild admin. Membership is not a
  dropbear key. See the trust contract.
- CAPPORT or welcome-page behavior.
- Bridging client L2 segments across nodes.
- Encrypting the one SSID. That is a later owner-committed plan. This
  change does not do it by itself.
