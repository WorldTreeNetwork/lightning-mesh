# Learnings

Append-only. One line per hard-won fact. Dated, with a file reference.

- 2026-08-31 — This repo had no `openspec/`; `ready.py` / `run.py --until fold` stop empty until a change is actually scaffolded. (`openspec/`, `/Users/dukejones/.agents/skills/ready/scripts/ready.py`)
- 2026-08-31 — Captive portal copy is two buttons: IdentiKey CTA + “Just the internet, please”. There is no “No thanks” — that was the same pass-through. Direct fix, not a foldable change. (`crates/mjolnir-hello/src/portal.rs`)
- 2026-08-31 — Factory client SSID ⚡ and live-fleet `Lightning Mesh` must stay two knobs. Changing the `setup-wireless.sh` default does not rename the air until `mjolnir-apply` `RUN_WIRELESS=1` with that env. Keep `wireless.env.example` as the live override. (`deploy/openwrt/setup-wireless.sh`, `add-client-network-name`)
- 2026-09-04 — Lightning Admin must wrap `update-fleet.sh --wireless`; discovery still has no SSH. Admin is another writer of `mjolnir-apply`, not a live UCI mutation. Field apply on live nodes was not exercised this session. (`admin/`, `add-lightning-admin-network-name`)
- 2026-09-04 — Do not wrap Admin network-name in `update-fleet.sh`: local `mjolnir-meshd-aarch64` was 2026-08-07 and would downgrade live boxes; overlay `10.254.x` is unreachable from the house WAN LAN; all-skipped must not be OK. Wireless-only `apply-network-name.sh` + `LIGHTNING_FLEET_SSH`. wr3000s-a WPS window dies on `fw4 reload`/`wifi reload` (no Allow-SSH-WAN); ap3000 survived via leftover UCI Allow-SSH-WAN. (`deploy/openwrt/apply-network-name.sh`, ap3000 OK idempotent Lightning Mesh)
- 2026-09-04 — wr3000s-a wireless-only apply OK (SSID Lightning Mesh, binary unchanged) but br-mesh had no 10.254 until meshd restart; health gate passed on hello HTTP not overlay. ap3000 WAN SSH survived via leftover UCI Allow-SSH-WAN; wr3000s-a WPS window died on fw4/wifi reload. (`lq1`-adjacent)
