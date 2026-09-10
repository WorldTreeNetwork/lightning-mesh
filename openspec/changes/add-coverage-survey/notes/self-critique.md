# self-critique add-coverage-survey

Author tab (Grok). Not advise. No ADVISE banner.

- Pinned directory.json as live store because it already grows additively
  and hello already reads it. Risk: CRDT stamp-write path is not designed
  here; `add-node-coordinates` has to find the actual book, not invent a
  second store.
- Refused radio.json v1 extension so the existing topology clients do not
  break. Join-by-id is extra work for `/mesh`.
- Compass vs AI Camera split is the load-bearing hardware call. If Duke
  later fuses form factors, `add-survey-world-model` revisits; this ADR
  should not grow lidar on T-RGB.
- DreamBall as snapshot (not live) keeps protocol blast in Dreamball
  attributes instead of a fourth axis. Field names deferred to
  `add-dreamball-coverage`.
