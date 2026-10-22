# add-wifi-internet-uplink

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

A house whose internet arrives on someone else's Wi-Fi needs one node to
join that network as a station and feed the mesh, then go back to the
previous configuration when the join is wrong. A DHCP lease on the station
is not evidence that a downstream client has internet.

## What

- Bind uplink credentials to an authorized candidate node and radio.
- Commit the station path only as a network-change transaction whose receipt
  does not treat DHCP as downstream internet.
- Restore the previous plan when credentials, health, or the receipt are bad.
- Keep the passphrase out of receipts and hello.mesh responses.

## Impact

- Capabilities: ADDED wifi-internet-uplink.
- ADRs: none.
- Work node: mjolnir-mesh-bf7.2. Depends on folded network-change-transactions
  and the closed radio inventory. Activation inherited from Duke's
  `activate ai0`. No live radio writes. Which radio is a good receiver stays
  `mjolnir-mesh-lpv`.
- Paid transit stays parked.

## User journey & surfaces

An owner, from the future Manage network surface, selects one visible upstream
network and one eligible node, then enters credentials for that candidate.
A downstream Ethernet client is not marked online merely because the station
got a lease. Wrong credentials restore the previous committed configuration.
The house's own client SSID is not offered as upstream. An open SSID that
nobody selected is not joined.

## Out of scope

- Live association on the fleet.
- Choosing AP3000 or any other model as the winning receiver.
- Resident or guest passphrase networks (`mjolnir-mesh-ai0.7`).
- Paid or metered transit policy.
- Port-role profiles (`add-wired-node-profiles`), except that the station
  radio role must not contradict a committed profile.
