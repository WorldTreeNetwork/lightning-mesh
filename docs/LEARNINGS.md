# Learnings

Append-only. One line per hard-won fact. Dated, with a file reference.

- 2026-08-31 — This repo had no `openspec/`; `ready.py` / `run.py --until fold` stop empty until a change is actually scaffolded. (`openspec/`, `/Users/dukejones/.agents/skills/ready/scripts/ready.py`)
- 2026-08-31 — Captive portal copy is two buttons: IdentiKey CTA + “Just the internet, please”. There is no “No thanks” — that was the same pass-through. Direct fix, not a foldable change. (`crates/mjolnir-hello/src/portal.rs`)
- 2026-08-31 — Factory client SSID ⚡ and live-fleet `Lightning Mesh` must stay two knobs. Changing the `setup-wireless.sh` default does not rename the air until `mjolnir-apply` `RUN_WIRELESS=1` with that env. Keep `wireless.env.example` as the live override. (`deploy/openwrt/setup-wireless.sh`, `add-client-network-name`)
