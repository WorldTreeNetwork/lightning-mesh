# Tasks

- [ ] hello.mesh stamp UI: nearby-narrowed node list, default = associated node, pick + GPS (geolocation or typed)
- [ ] POST signed stamp to hello on the LAN gateway; verify; spool; meshd ingest
- [ ] Reject missing node_id, unknown node, missing GPS-and-empty-typed, bad signature (write nothing)
- [ ] Fixture test: pick+GPS writes; cancel writes nothing; no-GPS writes nothing
- [ ] Rank list by associated-first, then nearby radio strength when known

Not owed here:

- Compass GPS HAL
- Coverage walk / heatmap
- DreamBall snapshot
- `/mesh` paint
- NFC / BLE / ESP-NOW on routers
