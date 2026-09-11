# advise add-node-coordinates — 2026-09-10

> **ADVISE:** send-back
> **READER:** opus-5-change-review
> **SPAWN:** .spawns/mjolnir-mesh-6hn.2-1789087719-11607-0749651b

Reader pass on an ACTIVE BUILD change. Rigor: change. Read-only —
nothing implemented, nothing folded, banner untouched.

## Blind take (written before tasks.md / spec delta)

From Why, the living `mesh-coverage` spec, and cited code only:

1. Pin: a new book `crates/mjolnir-mesh/src/crdt/coordinate.rs` keyed by
   subject `node_id`, with a `GossipMessage` variant appended LAST so existing
   discriminants are undisturbed, plus the mirror of
   `old_enum_decode_of_node_name_announce_bytes_errors` — postcard has no
   forward-compat skip and the recv-loop log-and-skip is the only thing keeping
   an un-upgraded node alive.
2. Pin: the apply path must NOT inherit node_name's integrity arm
   (`entry.node_id != *node_id` → drop). A phone stamps someone else, so
   subject != announcer by design. That deletion has to be a loud doc comment,
   or a later reader "restores the missing check" and silently kills every
   third-party stamp.
3. Refuse: reusing `NodeNameBook` / `NodeNameAnnounce` for coordinates. The
   authority model is different; overloading makes the existing forged-subject
   test incoherent and couples an operator handle to a survey claim.
4. Pin: make the merge a total order. `HLC` already carries `node_id` as its own
   tiebreak, so `merge_coordinate` should be pure LWW on `stamped_at` and use
   the stamper only for attribution/display. Two independent comparison keys
   (time AND stamper) invite divergent replicas.
5. Pin: convergence. `sync.rs` is broadcast-only — there is no state exchange.
   node_name converges because the SUBJECT re-announces on cadence. A stamp's
   author is a transient phone, so the subject node must be designated the
   re-announcer of its own last-known stamp, or a node that joins later never
   learns any coordinate and the book decays to whoever was listening.
6. Pin: projection is additive `Option` + `skip_serializing_if = "Option::is_none"`
   on both `DirectoryNode` and `DirectoryNeighbor`, mirroring `name` exactly.
   The unmarked invariant must be asserted on serialized JSON (no `lat`/`lon`
   keys), not on the Rust struct — a `None` field still satisfies a struct
   assertion while serializing `null`.
7. Pin: the acceptance gate is `cargo test -p mjolnir-mesh --lib -- coordinate`.
   That filters by name, so tests must literally contain "coordinate" or the
   gate passes vacuously on zero tests. Worse: `DirectoryNeighbor` lives in the
   bin crate, which cannot compile natively on macOS — so the "unmarked omits
   the key" invariant needs a lib-crate home (serialize a lib-side shape) or
   the stated acceptance does not actually cover the headline claim.
8. Refuse: any authentication claim landing in this node. Gossip verifies no
   signatures. If meshd ingests a hello-spooled stamp and re-gossips it, any
   mesh peer can relocate any node on the map. Ship this as
   untrusted-but-attributed and write that down; do not let the living spec's
   "hello verifies the signature" sentence imply meshd verified anything.
9. Refuse: raw `f64` lat/lon in the entry. The sibling books derive
   `PartialEq, Eq`; `f64` is not `Eq`, and float wire encoding is a bad
   equality/idempotence substrate for a CRDT. Fixed-point micro-degrees
   (`i32`, 1e-7 deg ≈ 1.1 cm) is deterministic, postcard-compact, and keeps
   `Eq`.
10. Tradeoff: the packet says the join to `radio.json` is `backhaul_addr`, but
    the book should still be keyed by `node_id` — addresses are derived from the
    node id and re-derivable, so keying on them makes the book re-key on any
    addressing change. Accept node_id as key, backhaul_addr as the consumer's
    join. Open concern the author must answer: there is no unstamp/tombstone —
    a wrong coordinate is only correctable by a newer one, and nothing bounds
    staleness when a router physically moves.

## Comparison with tasks.md and the spec delta

The delta and the blind take agree on the shape of the thing: additive optional
stamp fields, `skip_serializing_if`, unmarked omits the keys, join on
`backhaul_addr`, `radio.json` untouched. Take lines 6 and 10 are the delta's
own scenarios almost word for word, and the parent capability already pins
LWW and the no-`(0,0)` rule. That part needs no defence.

Where I revise my own take: line 4 argued for HLC as the merge key. The delta
says "Unix stamp time", and the delta is right — the author of a stamp is a
phone, which has no HLC and is not a mesh node. An HLC minted by the ingesting
daemon would order by ingest, not by observation, and two nodes ingesting the
same phone stamp would mint different HLCs and never converge. Foreign-authored
wall-clock plus stamper identity is the correct substrate. The concern survives
in weaker form, as (B) below.

Five things the author must answer before this is buildable. None require a
redesign; each is a sentence or two of pinning.

**(A) The acceptance command cannot exercise the delta's headline scenario.**
The gate is `cargo test -p mjolnir-mesh --lib -- coordinate`. `DirectoryNode`
and `DirectoryNeighbor` are private structs inside
`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`, which `--lib` does not build at
all, and whose tests the packet says cannot compile natively on this host. So
"that neighbor object has no lat/lon keys" is unverifiable by the stated
command, and the gate would pass on a coordinate book that was never projected.
There is a clean fix with precedent: `crates/mjolnir-mesh/src/radio.rs` owns a
serialized JSON contract in the lib and asserts its shape with `serde_json`,
which is a plain dev-dependency, so default-feature lib tests can assert JSON.
Put the coordinate's serialized value type in `crdt/coordinate.rs` with its own
omission behaviour, test it there, and let the bin only compose it. That keeps
the fix inside `constraints.paths`.

**(B) The LWW tiebreak is not a total order.** The delta gives "Unix stamp time,
and stamper identity"; the living spec says "last-writer-wins on `stamped_at`
plus stamper identity". Neither says what happens when two stamps for one node
carry the same `stamped_at` — and phone clocks collide at second granularity.
Pin the comparison explicitly, e.g. newer `stamped_at` wins, ties broken by
lexicographically greater stamper id, identical pairs are `Unchanged`. Also
state the clock policy: the stamp time is the phone's claim, meshd does not
rewrite it, and a stamp from the future is not clamped. Without this, replicas
that receive the same two stamps in different orders hold different
coordinates forever and nothing detects it.

**(C) Nothing states who may stamp whom.** This is the one real departure from
the `node_name` pattern and neither the delta nor tasks.md mentions it.
`apply_node_name_message` drops any entry where `entry.node_id != *node_id`,
precisely because a node only announces its own name. A coordinate stamp
inverts that: the subject is never the author. An implementer working from
tasks.md alone will either copy the integrity arm, silently killing every
third-party stamp, or drop it without comment, in which case any mesh peer can
relocate any node on the map and the projection presents it as fact. The delta
needs one requirement sentence: the subject may differ from the stamper, the
stamper id is recorded as attribution only, and gossip verifies no signature —
so consumers treat a coordinate as an attributed claim, not an authenticated
one. The living spec's "hello verifies the signature, spools the claim" line
must not be read as meshd having verified anything.

**(D) No designated re-announcer, so the book does not converge.** `crdt/sync.rs`
is broadcast publish and receive; there is no anti-entropy state exchange.
`node_name` converges only because the subject re-announces itself on the
cadence at `mjolnir-meshd.rs:4404`. A stamp's author is a phone that has left
the network. Task 3 asks only that "a stamp on one node is visible to
neighbors", which a single broadcast satisfies while leaving every node that
boots or joins later permanently blank. Pin the subject node as the periodic
re-announcer of its own last-known stamp, carrying the original stamper and
time unchanged, or say explicitly that convergence for late joiners is deferred
and to which change.

**(E) Task 4 asks for a test the constraints forbid.** "radio.json fixtures
unchanged" would live in `crates/mjolnir-mesh/src/radio.rs`, which is not in
`constraints.paths`. Either widen the path or restate the task as what it
actually is: this change does not touch `radio.rs`, and the existing `ng9`
contract tests are the evidence.

Smaller notes, not blocking: the entry type should avoid raw `f64` if it mirrors
`NodeNameEntry`'s `Eq` derive, since `f64` is not `Eq` — fixed-point
micro-degrees keeps the derive and the wire deterministic. The new
`GossipMessage` variant must be appended last and needs the mirror of
`old_enum_decode_of_node_name_announce_bytes_errors`; every prior variant
carries that test and its doc comment about postcard's lack of forward-compat
skip. Key the book by `node_id`, not `backhaul_addr` — the address is derived
from the id, and `backhaul_addr` is the consumer's join, as the delta already
says.
