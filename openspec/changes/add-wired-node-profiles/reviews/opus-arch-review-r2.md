# Architecture review r2 — add-wired-node-profiles (ai0.6)

> **ADVISE:** accept
> **READER:** opus-arch-review
> **SPAWN:** .spawns/mjolnir-mesh-ai0.6-1792654293-88968-297469b2

Fresh pass after the r1 send-back (`reviews/opus-arch-review.md`, B1–B5).

## Blind take (written before opening design.md / tasks.md / the r1 review)

Sources: proposal, living `network-change-transactions` spec, household
design §2–§4, `crates/mjolnir-apply/src/{model,openwrt}.rs`,
`crates/mjolnir-mesh/src/babel/config.rs`, `deploy/openwrt/setup-wireless.sh`.

1. PIN: a profile compiles to one revision-checked plan; every refusal
   (role conflict, unqualified map) happens before `Prepared`.
2. PIN: one role per port/radio; a node-link is never in `br-lan` and never
   in an L2 bridge with the 802.11s radio (`mesh_fwding=1` + house switch = loop).
3. PIN: ports the profile does not name are `disabled`, not inherited `br-lan`.
4. PIN: babel must learn the node-link, and as a cable, distinct from the
   radio-backed `l2_interfaces` that kf7 deliberately renders `type wireless`.
5. PIN: the commit condition for a port/radio plan names management and mesh
   reachability, and says what happens while the adapter reports `Unknown`.
6. PIN: cable-loss mode is a typed field; releasing the radio must not strand
   a current wireless dependant without an explicit acknowledgement.
7. REFUSE: a "guess from board.json" map, any implicit WAN / default-export /
   `192.168.1.1/24` change, and change-scope sentences in the living spec.
8. TRADEOFF: `isolate` buys both client bands for a silent island on unplug;
   `reserve-mesh` keeps fallback at the cost of a client band. The owner must
   see which one they chose.
9. Scope: no live writes ⇒ decider + builder + renderer + sim/fake tests only.
10. Default profile must be a no-op on today's generated config.

## Comparison

| Blind item / r1 blocker | Where answered | Status |
|---|---|---|
| B1 node-link vs 802.11s loop (take 2) | design "Loop and port rules"; spec "Wired-core keeps the three domains apart" + scenario "Node-link joined to the mesh bridge"; tasks test | Answered |
| B2 how routing learns the node-link (take 4) | design: plan writes `mjolnir.node_link`, meshd renders as wired, renderer path in this change; tasks "Babel renderer reads `mjolnir.node_link`" | Answered |
| B3 required health (take 5) | design "Health (B3)": option (a), fake adapter, refuse `OpenWrtAdapter`; spec "Profile apply is a durable transaction"; bead `mjolnir-mesh-2mgv` exists and cites this change | Answered |
| B4 scope sentence in the living spec (take 7) | Removed from the spec delta; now in design "Not owed here" + tasks | Answered |
| B5 unassigned mapped ports (take 3) | design + spec + scenario "Unassigned mapped port" + tasks test | Answered |
| S1 stranding dependants (take 6) | spec: `isolate` refused without acknowledgement when a peer has no other path | Answered |
| S2 default no-op (take 10) | spec scenario + tasks test | Answered |
| S3 map location / marker | image data files by model, `qualified: true` only from `lpv`, fixtures `false` | Answered |
| S4 previous revision, profile id | spec wording + profile id in `mjolnir` UCI in the same plan | Answered |
| S5 cabling software cannot refuse | design: observations, never "healthy" | Answered |

All five blockers have a rule, a spec scenario or requirement, and a task.
My blind take adds no new blocker.

## Steelman against my take

- *"Take 4 risks re-litigating kf7."* No. kf7 chose `type wireless` because
  the `veth-mesh`/`br-mesh` "wire" rides a radio. A real Ethernet node-link
  is a different input. The design keeps it separate (`mjolnir.node_link`).
  That is the right split if the implementation keeps it separate (N1).
- *"Requiring probes that return `Unknown` makes the plan useless."* Yes,
  until `2mgv` lands, and that is intended. Without live probes, a port swap
  that cuts SSH must not commit. The design says so and refuses the real
  adapter instead of weakening the gate.
- *"An acknowledgement override for stranding peers is a footgun."* It is
  typed and explicit in the plan. The alternative is to forbid `isolate`
  whenever any wireless satellite exists, which would make `wired-core`
  unusable in partial-Ethernet houses. Acceptable.

## Non-blocking notes for the builder

- **N1. Keep `node_link` a separate renderer input.** Do not route node-links
  through `BabelConfigInputs::l2_interfaces`. That changes their metric to
  `type wireless` + RTT, or worse, changes the radio's metric to `type wired`.
  Add a renderer test with both inputs present.
- **N2. Define "mesh reachability" for `isolate`.** After the radio is
  released, mesh reachability means a babel neighbour over the node-link,
  not an 802.11s peer. State this in `2mgv`, or the probe will restore every
  correct `isolate` apply.
- **N3. Name `2mgv` in design "Health".** The bead exists and links back to
  this change. The design says only "a follow-up bead".
- **N4. Small wording drift.** Design "Apply" still says "restores the previous
  profile". The spec correctly says "previous revision". Align the design
  text when you next edit it.
- **N5. Backhaul band is island-wide.** `setup-wireless.sh` requires the same
  `BACKHAUL_BAND` on every node. The fleet is `5g` today, but `2g` is
  supported. Planning the 5 GHz default onto a `2g` node is a fleet-level
  change that splits an island, not a per-node no-op. The decider should
  refuse it, or run the no-op test per node against that node's own
  generated config.
- **N6. No `reserve-mesh` scenario.** The requirement text covers it ("the
  reserved radio SHALL stay off client AP"). A sibling to "Cable loss
  isolates" would make it testable.

None of these change the architecture. They can be handled during
implementation or by the folder.
