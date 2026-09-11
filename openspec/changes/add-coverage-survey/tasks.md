# Tasks

- [x] `openspec/changes/add-coverage-survey` proposal, design, steer, deltas
- [x] Send-back: rewrite call 3 — router proximity is Wi-Fi association + nearby client-BSSID RSSI, then operator pick + entered GPS; BLE/ESP-NOW is handheld-to-handheld only; routers gain no NFC, BLE, or ESP-NOW
- [x] Send-back: pin stamp ingress — signed claim to hello on the node's LAN gateway, spooled to meshd; hello verifies only; CRDT is the authority
- [x] Advise accept (other-family reader) before act — `reviews/2026-09-10-re-advise.md`
- [ ] Amend `ARCHITECTURE.md` with the five coverage-survey calls

Not owed here:

- Directory lat/lon — `add-node-coordinates`
- DreamBall payload — `add-dreamball-coverage`
- `/mesh` paint — `add-coverage-world-viz`
- Hello stamp UI / phone pick — `add-compass-node-mark`
- Sweep game — `add-coverage-sweep`
- World-model overlay — `add-survey-world-model`
- Compass GPS HAL
- Fold
