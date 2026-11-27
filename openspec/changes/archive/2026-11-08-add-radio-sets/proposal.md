# add-radio-sets

> **ACTIVE BUILD**

**Rigor:** architecture
**Work node:** mjolnir-mesh-i5m0
**Authority:** campaign grant 2026-11-07 on mjolnir-mesh-i5m0 (activate). Radiosets are a separate capability from wired port profiles.

## Why

A node can put a radio on mesh backhaul, a client AP, a station uplink, or nothing. `setup-wireless.sh` infers two bands and refuses a live station (z3th). `node-connection-profiles` already names radio roles but ties them to Ethernet profiles. Extra phys are last-wins. The house needs a reusable radioset: which phy does which function, projected by apply, without mixing port roles into the same document.

## What

- Add capability `radio-sets`: a named assignment of every phy on a model to mesh, client-ap, station, or disabled.
- Ship per-model radio maps (image data, like port maps). OpenWrt One is two phys on one MT7981 wmac (DBDC): 2.4 GHz 2×2 and 5 GHz 3-chain. Three MMCX jacks are MIMO, not a third mac80211 radio.
- Default radioset is the satellite split: 5 GHz mesh, 2.4 GHz client AP. Station is a legal function; this SKU does not assign it.
- `mjolnir-apply` / `setup-wireless.sh` project the radioset. A station the set assigns is preserved. A phy the map lists and the set omits is disabled.
- Split radio assignment out of `node-connection-profiles`. Wired profiles keep ports. A profile may name a radioset id; it does not list radio roles itself.

## Impact

- Capabilities: ADDED radio-sets; MODIFIED node-connection-profiles.
- ADRs: will amend ARCHITECTURE.md (radio plumbing is a radioset, not BACKHAUL_BAND last-wins).
- Follow-ons (not this change): preserve-station-apply (mjolnir-mesh-9d3o), live-uplink-probes (mjolnir-mesh-21rw). Hardware qualification stays mjolnir-mesh-lpv.

## User journey & surfaces

An operator on a cabled OpenWrt One, after stock install, has Ethernet WAN, 5 GHz 802.11s backhaul, and the Lightning Mesh client AP on 2.4 GHz. Applying the default radioset is a no-op on that split. Applying a set that puts two functions on one phy is refused before mutation. LuCI-edited `wireless` is still not canonical.

No new UI because `mjolnir-apply` with `RUN_WIRELESS=1` is already the projector (`mjolnir-settings`). Lightning Admin Internet-source UI is mjolnir-mesh-849.4.

## Out of scope

- Solstice Grove station on this OpenWrt One (steered: keep mesh + client AP; WAN is Ethernet).
- Candidate-bound Wi-Fi uplink commit on OpenWrtAdapter (mjolnir-mesh-21rw).
- USB/M.2 third-radio qualification (mjolnir-dongle ap2g stays the only validated extra AP).
- Live fleet radio writes in this change's act wave until advise accepts.
- Storing upstream passphrases in git or beads.
