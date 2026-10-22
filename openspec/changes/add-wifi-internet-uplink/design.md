# Wi-Fi internet uplink

## Pins

1. **Candidate.** A test or commit names one node id, one radio id, and the
   selected upstream SSID. The BSSID is recorded when the scan saw one, and a
   later association to a different BSSID for that SSID is a new candidate,
   not a silent reuse. Credentials are not tried on a node that was not
   authorized for that candidate.

2. **Receipt.** Success is a committed `network-change-transactions` outcome.
   A DHCP lease on the station is recorded as local evidence only. Downstream
   client internet stays unknown until a qualifying client-path observation
   exists. The transaction must not report committed-and-online from the lease.

3. **Rollback.** Wrong credentials, failed health, or an uncertain receipt
   restore the previous committed plan through the same journal. A new
   default route is not installed beside an existing house-gateway export
   unless that export change is inside the same revision-checked plan.
   AP3000's current `0.0.0.0/0` export is such an existing export. This
   change does not delete it as a side effect.

4. **Secrets.** The passphrase is stored by reference on the node. Receipts,
   controller-facing logs, and hello.mesh responses do not contain it.

5. **Refusals.** Do not join an open SSID that was not selected. Do not use
   the house client SSID (`Lightning Mesh` on this fleet, or the configured
   client SSID) as upstream. Do not bridge the station into `br-lan`.

6. **Profile fit.** If a committed node-connection profile already assigned
   that radio, the uplink plan may use it only when the role is `station`.
   Otherwise the plan is refused until the profile changes in the same
   transaction or an earlier committed one.

## Not owed here

- A live scan or association. Software builds the plan and the refusal rules.
- Rating which physical radio hears the upstream best (`mjolnir-mesh-lpv`).
- Metered-transit consent. Parked until a separate change.
