# Architecture review — add-wired-node-profiles (ai0.6)

> **ADVISE:** send-back
> **READER:** opus-arch-review
> **SPAWN:** .spawns/mjolnir-mesh-ai0.6-1792653881-83220-d2eeb210

## Blind take (written before opening design.md / tasks.md)

Sources: proposal Why/What, living `network-change-transactions` spec,
household design §2–§4/§6, `crates/mjolnir-apply/src/{model,engine,openwrt}.rs`,
`crates/mjolnir-mesh/src/babel/config.rs`, `deploy/openwrt/setup-wireless.sh`.

1. PIN: a profile is a pure, typed validator that emits one `Plan` over the
   existing `UciConfig` allowlist (wireless/network/firewall/mjolnir); refusal
   happens before `Prepared`, so a refused profile never touches the journal.
2. PIN: per-port role exclusivity (WAN | node-link | client | disabled), and a
   node-link port is NEVER enslaved to `br-lan`, and NEVER to `br-mesh` while
   the 802.11s radio (`mesh_fwding=1`) is kept — that is an L2 loop via the
   house switch. The node-link must be its own routed babel interface.
3. PIN: babeld today renders every L2 interface `type wireless` with RTT
   penalties; a wired node-link needs a distinct babel interface class or the
   "prefer validated wire" outcome is not guaranteed.
4. PIN: cable-loss behavior is a named enum on the profile (release-radio /
   reserve-radio), not inferred at runtime; reserve-radio keeps the existing
   satellite split byte-for-byte.
5. PIN: unqualified model ⇒ refusal names the missing port map; no
   "guess from board.json" fallback, no default wired-core map.
6. PIN: the default satellite profile must render a revision identical to
   what `setup-wireless.sh BACKHAUL_BAND=5g` produces today (no-op apply).
7. REFUSE: committing a wired-core plan on `AppliedRevision` alone. The OpenWrt
   adapter reports Management/MeshReachability as Unknown, so either the plan
   requires them (and therefore always restores today) or it does not (and a
   port swap that cuts SSH commits). The change must say which and why.
8. REFUSE: any profile that changes WAN placement or gateway default-route
   export implicitly (proposal already excludes AP3000 changes — keep it typed).
9. TRADEOFF: release-radio buys both client bands at the cost of a silent
   island on cable loss; the spec should require the receipt/UI to state that
   isolation, since the transaction cannot detect a future unplug.
10. Scope check: "no live radio writes" + hardware in `lpv` means this change
    can only land model + validator + plan rendering + sim tests; anything
    claiming apply-level health is out of reach until adapter probes exist.

## Comparison with design.md, tasks.md and the spec delta

| Blind item | Design / spec answer | Status |
|---|---|---|
| 1 Pure decider → one plan, refuse before mutation | "Refusal happens before mutation"; ships decider + plan builder only | Answered |
| 2 Node-link never shares an L2 bridge with the 802.11s radio | Silent. `wired-core-with-fallback` keeps a mesh radio *and* a node-link but no rule says where the node-link attaches | **Open — blocking** |
| 3 Babel on the node-link as a wired interface | Silent. Only "client ports do not run babel" | **Open — blocking (with 2)** |
| 4 Cable loss is a named mode | `isolate` / `reserve-mesh` per profile | Answered |
| 5 Unqualified model ⇒ refuse with the missing map named | Spec scenario. AP3000 observation is explicitly not a map | Answered; where maps live is unstated (S3) |
| 6 Default profile is a no-op on today's fleet | "Matches the current fleet split", but no no-op requirement | Open (S2) |
| 7 Health a profile plan must prove before commit | Silent | **Open — blocking** |
| 8 No implicit WAN or gateway-export change | Spec requirement + scenario; tasks test it | Answered |
| 9 Release-radio isolation is shown to the owner | Only in the proposal's user journey | Minor (S4) |
| 10 Scope = model + validator + plan + tests | Matches the design | Answered, but leaks into the living spec (B4) |

The design's role rules and the tasks' loop-refusal tests are good. They
pin client babel, node-link DHCP, WAN in `br-lan`, cross-node client
bridges, the AP3000 `192.168.1.1/24` add, and silent default-export
deletion. The gaps below are about the loop the Why actually names, and
about what "commit" means for this plan.

## Blocking (send-back)

**B1. Where the node-link lives and the 802.11s loop.** `setup-wireless.sh`
bridges the mesh point into `br-mesh` with `mesh_fwding=1`, a flat L2 island.
In `wired-core-with-fallback`, two nodes each keep that radio *and* plug a
node-link into the house switch. If the node-link joins `br-mesh` (the obvious
way to "give babel the wire"), the switch and the 802.11s island form an L2
broadcast loop. Household design §4 says this outright: "the current 802.11s
forwarding island makes naive bridging unsafe." The proposal's Why names loops.
Today the role rules cannot refuse this plan. Required:
- A role rule: a node-link port SHALL NOT be bridged with a mesh radio. It is
  its own routed L3 interface (or a separate bridge with no radio member).
- A test in tasks: "node-link enslaved to `br-mesh` while a mesh radio is
  kept" is refused.

**B2. How routing learns the node-link.** babeld's config is rendered by
meshd (`crates/mjolnir-mesh/src/babel/config.rs`), not by UCI. Every
`l2_interfaces` entry is `type wireless` with RTT penalties. The plan builder
only writes the four allowlisted UCI files. The design must name the
`mjolnir` UCI key that tells meshd which interfaces are node-links, and must
say that babel runs on them as a wired-class interface. Without that, a
`wired-core` apply leaves a node with a cable but no route over it. With
`isolate`, that node is an island right after a "successful" commit. If the
meshd side is a follow-up, say so and file the bead. Do not leave it implied.

**B3. Which health the profile plan requires.** `Plan.required_health` is part
of what the plan builder emits. Today `OpenWrtAdapter::check_health` returns
`Unknown` for every kind except `AppliedRevision`, and the engine restores on
any required kind that does not pass. So there are only two possibilities:
- (a) Profile plans require `ManagementReachability`/`MeshReachability`.
  Every apply restores until adapter probes exist.
- (b) They require only `AppliedRevision`. Then a port reassignment that cuts
  the management path or the node-link commits as healthy.

Both may be acceptable for a change with no live writes. The design must pick
one, pin it in a test, and name the bead that adds the probes. I recommend
(a) for any plan that touches `network` port roles or a mesh radio. Leaving
it unstated means the folded spec promises a durable transaction whose commit
condition nobody chose.

**B4. Change scope leaked into the living spec.** "This capability's software
SHALL NOT perform a live radio write" will outlive the fold. But applying a
profile *is* a wireless UCI change plus a radio reload, which is the
capability's whole purpose. Move that sentence to the proposal or tasks
("this change ships no live writes"). In the spec, replace it with a statement
of what a radio-touching profile plan must require (see B3).

**B5. Ports the profile leaves unassigned.** The spec says "each *used*
port". On stock OpenWrt the remaining LAN ports sit in `br-lan`. In a
`wired-core` house, an unassigned port that gets cabled to the house switch
bridges the node's client LAN onto the node-link segment. That is exactly the
"treat every Ethernet link as one LAN" failure in the Why. Required: under a
qualified map, every physical port in the map gets an explicit role, and
unassigned means `disabled`, not left in `br-lan`. Alternatively, refuse a
profile that leaves a mapped port unassigned. Add a test.

## Should answer (not blocking alone)

- **S1. Dependants of a released radio.** In `wired-core`, "both client bands
  only if no radio is required for mesh" leaves open who decides "required".
  Household §4: exclude a change that would isolate dependants. The decider is
  pure and can take observed mesh neighbours as input. It should refuse
  `isolate` (or require an explicit acknowledgement) when a current mesh peer
  has no other path. Otherwise the cable-loss tradeoff is not the only cost:
  releasing the radio strands a wireless satellite immediately, with no
  cable loss at all.
- **S2. The default profile is a projection, not a regeneration.**
  `setup-wireless.sh` carries channels, FT, `CLIENT_AP_2G`, `MESH_KEY` and
  BACKHAUL_BAND. Applying `ethernet-gateway-satellites` to an unchanged
  current node should give `proposed_revision == expected_revision` (no-op).
  A plan builder that re-renders `wireless` from defaults would bounce radios
  and drop per-node knobs. Add a no-op test against a fixture of today's
  generated configs.
- **S3. Where qualified port maps live.** Is it a file in the image, `mjolnir`
  UCI, or a crate table? Who writes it (`lpv` output)? Tests need fixture maps
  that cannot be mistaken for qualified ones. Name the format, the location,
  and the "qualified" marker.
- **S4. "Previous profile" vs previous revision.** Fleet nodes today have no
  recorded profile. The transaction restores the previous file *revision*.
  Reword the spec as "previous revision remains committed" so legacy nodes
  are covered. Also record the profile id in `mjolnir` UCI so the next
  decider run can read it back.
- **S5. Cabling the software cannot refuse.** A client port and a node-link
  on the same unmanaged switch bridges domains whatever the UCI says. Out of
  scope for refusal, but worth one line: observed babel hellos on a client
  port, or foreign DHCP on a node-link, should surface as an observation for
  `lpv`/UI, never as "healthy".

## Steelman

- *"B1/B5 are unreachable because no model is qualified, so wired-core is
  always refused."* True for this change. But the decider and its role rules
  are what `lpv` qualification unlocks, and the spec gets folded as living
  law. Qualification is hardware evidence. It should not also have to discover
  that the topology rules allow a loop. The rules are cheap to pin now and
  hard to retrofit after a map lands.
- *"B3 is moot without live applies."* The plan builder's output includes
  `required_health`, so the builder is choosing an answer anyway. The choice
  should be explicit and tested.
- *"B2 is meshd work, out of scope."* Acceptable, if the design says so and a
  bead exists. The send-back asks for the dependency to be named, not built.
- The design rightly refuses to treat the AP3000 `eth0`-as-WAN observation as
  a map. It keeps station roles owned by `bf7.2`, and keeps automatic
  optimization out. Those are the right calls and need no change.

## What would flip this to accept

B1–B5 answered in design.md and the spec delta. That means one added role
rule, one named meshd/UCI interface (or a filed bead), one stated
`required_health` policy, the scope sentence moved out of the spec, and an
unassigned-port rule. Add the matching tests to tasks.md. S1–S5 can be
answered inline or deferred to beads.
