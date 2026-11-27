# add-live-uplink-probes

> **ACTIVE BUILD**

**Rigor:** change
**Work node:** mjolnir-mesh-21rw
**Authority:** campaign grant 2026-11-07 on mjolnir-mesh-i5m0. 9d3o folded. OpenWrt One field map stays Ethernet WAN (no Solstice Grove STA on this SKU).

## Why

`submit_uplink_plan` only accepts `ProvesUpstream`. `OpenWrtAdapter` does not implement it: `check_health` records Unknown for every reachability kind, so a station plan cannot commit on metal. A DHCP lease must still not mean client internet. Wrong PSK must restore without demanding upstream evidence.

## What

- `OpenWrtAdapter` implements `ProvesUpstream`.
- Apply reloads wireless after UCI write. Restoration proves `ManagementReachability` and `AppliedRevision` only (`check_restoration`).
- Health before commit observes association, address, route, and upstream-bound DNS+HTTPS on the station iface, plus mesh and management reachability.
- Secret file is snapshotted and restored with the transaction; receipts omit passphrase bytes.
- Tests use injectable probes. Field RF qualification stays `mjolnir-mesh-lpv`. Do not join Solstice Grove from this OpenWrt One.

## Impact

- Capabilities: MODIFIED wifi-internet-uplink.
- ADRs: none.

## User journey & surfaces

An operator (or later Admin Internet-source) submits a candidate-bound station plan. On a node whose radioset assigned that phy station, the durable transaction associates, proves upstream DNS+HTTPS, then may export default. Wrong PSK restores the previous revision. Receipts do not leak the passphrase.

No new UI because `mjolnir-txn` / `mjolnir-apply` is the commit path. Lightning Admin Internet-source is mjolnir-mesh-849.4.

## Out of scope

- Applying a station uplink on openwrt-one (steered: 5 GHz mesh + 2.4 GHz client AP, WAN Ethernet).
- lpv hardware qualification of a live join.
- Admin Internet-source UI (849.4).
- Changing radioset assignment (radio-sets, folded).
