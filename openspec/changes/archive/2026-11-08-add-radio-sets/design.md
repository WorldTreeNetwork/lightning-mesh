# Radio sets

## Shape

A **radioset** is a named map from phy id → one function: `mesh`, `client-ap`, `station`, `disabled`. It is not a wired node-connection profile. Ports stay in `node-connection-profiles`.

Committed store (replay without env): `mjolnir.radioset.id`, optional `variant` (`2g` only when not the 5 GHz default), and a resolved `function` table keyed by map `id`. Definitions live as image data next to radio maps (`satellite-split`, `satellite-split-2g`). Hardware maps do not store functions. A profile-named id wins over `BACKHAUL_BAND`. `BACKHAUL_BAND=2g` selects `satellite-split-2g` only on first adoption. Absent id is `satellite-split` (today's no-op). Unknown id is refused.

`setup-wireless.sh` today picks the last `band=2g` and last `band=5g` device and FATALS on `mode=sta`. The plan builder reads the radioset instead. The shell SHALL NOT commit UCI or `wifi reload` outside the durable transaction. Live radioset **changes** still refuse `OpenWrtAdapter` until probes exist. Preserving an **owned** station in the plan is this change; making the metal shell not FATAL that station is mjolnir-mesh-9d3o.

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

Identity: `id` is the UCI wifi-device name; `path` is hardware; `band` is a constraint. Match inventory by path, then require UCI name == id. Two 5 GHz phys are not interchangeable. Unmapped detected phys are unmanaged (dongle) unless they already carry a managed iface. Mapped-but-omitted is disabled.

`qualified: true` is only `mjolnir-mesh-lpv`. Unqualified maps still identify phys for the default set. OpenWrt One evidence (2026-11-07): one DT node `wifi@18000000` (`mediatek,mt7981-wmac`), driver `mt798x-wmac` / `mt7915e`, two wiphy (`phy0` 2.4 TX/RX 0x3, `phy1` 5 GHz configured 0x1c). Firmware `kmod-mt7981-firmware`. Not a missing third radio.

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

Same durable transaction rules as profiles: snapshot, health when mesh radio changes (`ManagementReachability` + `MeshReachability`), restore the previous **resolved** radioset record on failure. Until `OpenWrtAdapter` has those probes, the builder tests against a fake adapter and refuses submit of a radioset change to the live adapter. `clientap2g` stays rendered-disabled when that phy is not client-ap. Stale enabled ifaces on a disabled phy are disabled in the plan.

Wired-core `isolate` still requires `acknowledge_stranded_peers` when `wireless_only_peers` is non-empty (`node_profile.rs`). That check is composed profile+radioset validation, not replaced by health probes.

## Not owed here

- Implementing ProvesUpstream / Solstice Grove.
- Claiming the One radio map is `qualified: true`.
- Admin Internet-source UI.
