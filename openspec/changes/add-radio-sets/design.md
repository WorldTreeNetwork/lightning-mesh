# Radio sets

## Shape

A **radioset** is a named map from phy id → one function: `mesh`, `client-ap`, `station`, `disabled`. It is not a wired node-connection profile. Ports stay in `node-connection-profiles`. A node records `mjolnir.radioset.id` (and the profile may name that id). Apply projects both in one network-change transaction when both change, or radioset-only when ports do not.

`setup-wireless.sh` today picks the last `band=2g` and last `band=5g` device and FATALS on `mode=sta`. The projector reads the radioset instead. `BACKHAUL_BAND` remains a way to *select* the default satellite radioset (5g mesh vs 2g mesh), not a discovery loop.

## Model map

Image data: `/usr/share/mjolnir/radio-maps/<model>.json`.

```json
{
  "model": "openwrt,one",
  "qualified": false,
  "phys": [
    {"id": "radio0", "band": "2g", "path": "platform/soc/18000000.wifi"},
    {"id": "radio1", "band": "5g", "path": "platform/soc/18000000.wifi+1"}
  ]
}
```

`qualified: true` is only `mjolnir-mesh-lpv`. OpenWrt One evidence (2026-11-07): one DT node `wifi@18000000` (`mediatek,mt7981-wmac`), driver `mt798x-wmac` / `mt7915e`, two wiphy (`phy0` 2.4 TX/RX 0x3, `phy1` 5 GHz configured 0x1c). Firmware `kmod-mt7981-firmware`. Not a missing third radio.

## Default set

Id `satellite-split`: 5 GHz phy → mesh, 2.4 GHz phy → client-ap, anything else → disabled.

OpenWrt One field map is this set. Ethernet WAN is the uplink (eth0). Station is absent on this board.

## Refusals (before mutation)

- Two functions on one phy.
- Mesh + AP on the same phy (oaq / WR3000S field observation; default stays off).
- Station on a phy whose committed function is mesh unless the same transaction retargets mesh.
- Station credentials used on a phy this radioset did not mark station (`wifi-internet-uplink`).
- A mapped phy omitted from the set is disabled, not left as an unmanaged AP.
- Bridging a station iface into `br-lan`.

## Apply

Same durable transaction rules as profiles: snapshot, health when mesh radio changes (`ManagementReachability` + `MeshReachability`), restore previous radioset on failure. Until `OpenWrtAdapter` has those probes, the builder tests against a fake adapter and refuses submit to the live adapter (same pattern as wired profiles).

## Not owed here

- Implementing ProvesUpstream / Solstice Grove.
- Claiming the One radio map is `qualified: true`.
- Admin Internet-source UI.
