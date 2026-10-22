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

## Required health (advise 2026-10-22 S1)

A station plan requires `UpstreamAssociation`, `UpstreamAddress`,
`UpstreamRoute`, `UpstreamBoundDnsHttps`, `MeshReachability`, and
`ManagementReachability`. Restoration proves `ManagementReachability` and
`AppliedRevision` only. It does not require the upstream observations, so a
wrong passphrase can return to the previous revision instead of sticking at
recovery-required for lack of upstream evidence.

The builder is tested on a fake adapter. It refuses submission to
`OpenWrtAdapter` until that adapter can report those observations and can
reload inside apply and restore. That live path is a follow-up, not this
change.

## Secret file (S2)

The passphrase lives outside the four revisioned UCI files, at
`/etc/mjolnir/secrets/uplink/<candidate-id>`, mode 0600. The transaction
snapshots that file in a root-only journal sidecar. Controller-visible
revisions and receipts are computed without those bytes, so a guess cannot
be confirmed from a receipt. Restore puts the previous secret file back
before the result is restored. A reused reference name cannot point a
restored config at a newer secret.

## First gateway (S3)

The mesh default-route export is installed only after
`UpstreamBoundDnsHttps` passes. A DHCP lease, including a captive portal,
does not install or redistribute `0.0.0.0/0`. Failure restores before that
export is announced.

## Which networks (S4, S5)

Refuse every BSSID a mesh node advertises: house client, guest, captive,
dongle AP, and per-node SSID. SSID match alone is not enough. The station
section has exactly one network and no fallback list. Refuse a radio whose
committed wireless config is mesh mode, whether or not a profile exists.
An upstream subnet that overlaps `10.42.0.0/16`, `10.254.0.0/16`, or the
node's identity-derived ULA is a restore trigger, not a NAT exception.

## Not owed here

- A live scan or association. Software builds the plan and the refusal rules.
- `OpenWrtAdapter` probes and reload-during-apply.
- Rating which physical radio hears the upstream best (`mjolnir-mesh-lpv`).
- Dongle hotplug versus the transaction flock (separate bead).
- Supplicant-reason strings in receipts (live-path follow-up).
- Metered-transit consent. Parked until a separate change.
