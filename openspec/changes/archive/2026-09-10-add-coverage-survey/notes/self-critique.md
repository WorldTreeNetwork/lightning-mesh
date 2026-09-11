# self-critique add-coverage-survey

Author tab (Grok). Not advise. No ADVISE banner.

- First pass pinned BLE/ESP-NOW tap. Fable send-back: compass BLE is
  disabled; routers have no ESP-NOW peer. Human: routers are Wi-Fi;
  compass has no GPS; phone picks nearby + enters GPS. Call 3 rewritten.
- Stamp ingress now matches identity/name-claim: hello verifying spool,
  meshd CRDT authority, control API stays loopback.
- Join key pinned to `backhaul_addr` (only overlap of radio.json and
  directory.json). `mesh_mac` on directory is optional later.
- v1 surveyor is the phone, not the compass. Compass GPS is later.
- Privacy: full WGS84 on directory; ball export explicit. Staleness:
  LWW + viewer ages. Both [AUTO] after send-back notes.
