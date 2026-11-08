# add-preserve-station-apply

> **ACTIVE BUILD**

**Rigor:** change
**Work node:** mjolnir-mesh-9d3o
**Authority:** campaign grant 2026-11-07 on mjolnir-mesh-i5m0 (activate). i5m0 closed; radioset living spec folded.

## Why

`setup-wireless.sh` still FATALS on `mode=sta` in the last-wins template (z3th). The radioset branch already preserves an owned station, but `mjolnir-apply` `RUN_WIRELESS=1` does not export the committed `mjolnir.radioset.function` table, so a metal re-apply can miss that branch and clobber or refuse a live STA.

## What

- `mjolnir-apply` exports `RADIOSET_FUNCTIONS` from the committed radioset record before invoking `setup-wireless.sh`.
- `RUN_WIRELESS=1` never enters the last-wins template when a committed radioset table exists, or when a live STA is present without a table (refuse, do not clobber).
- Owned station iface and mesh/client-ap radios stay as the radioset assigned them.

## Impact

- Capabilities: MODIFIED radio-sets.
- ADRs: none.

## User journey & surfaces

An operator re-runs `install-node.sh --wireless` / `mjolnir-apply` with `RUN_WIRELESS=1` on a node whose radioset assigns station. The station iface remains; mesh and client AP are unchanged. No FATAL z3th.

No new UI because `mjolnir-apply` is already the projector (`mjolnir-settings`).

## Out of scope

- Live `ProvesUpstream` commit (mjolnir-mesh-21rw).
- New station association or passphrase handling (`wifi-internet-uplink`).
- OpenWrt One Solstice Grove STA (steered off; WAN is Ethernet).
