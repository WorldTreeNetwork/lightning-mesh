---
bead: mjolnir-mesh-e21.9
status: design (2026-07-04)
summary: one staleness + tombstone-GC mechanism for all self-announced gossip
  lanes — services (ServiceEntryV2), the .mesh name projection, and the 0yb
  address book. Liveness rides an EPHEMERAL per-node heartbeat beacon (no flash),
  separate from the durable CRDT. Local-only soft-state, no gossiped watermark,
  CRDT-safe.
---

# Self-announced lane staleness (e21.9)

Sprint-002 shipped the self-announced lanes with **no expiry** (a documented
MVP limitation): a node that goes offline leaves its published names resolving
to an unreachable IP (black-hole), `unpublish` tombstones are retained unbounded
(no GC), and stale address-book entries never age out. This is the one design
that fixes all three, since they share one substrate: self-announced,
HLC-stamped, re-announced every anti-entropy tick.

Distinct from `99f` (subnet-claim reclamation), which needs a liveness *probe*.
These lanes are self-announced with periodic re-announce, so the staleness
signal is simpler: **a missing re-announce IS the liveness signal — no Iroh
probe needed.**

## The core reframe: liveness is not durable state

The naive fix — re-stamp each entry's HLC every tick so "the clock stopped
moving" means "the origin died" — works, but it forces the persisted CRDT to
change every tick, rewriting `.state` files to router flash every 20s
indefinitely (the churn tracked by `7bf`). That is the wrong plane. **Liveness
is momentary, per-node, and rebuilt on boot; it must not live in the durable
CRDT.** Once liveness rides a separate ephemeral channel, the durable book only
persists on *real* change and the flash problem disappears.

So this design splits two clocks that were being conflated:

| | **Persisted HLC** (on the CRDT entry) | **Ephemeral heartbeat** (the beacon) |
|---|---|---|
| Job | order *data writes* for merge / conflict | prove *recency of contact* |
| Needs | total order across nodes → `wall_clock + counter + node_id` | "did it advance" + **local receive-time** |
| Durability | must persist — it's the arbitration record | must **not** persist — it's momentary |

The heartbeat is deliberately *weaker* than the HLC beside it: the timestamp
that decides staleness is stamped **locally by the receiver** (`local_now −
last_seen`), never carried in the beacon — so the beacon needs no wall clock,
and clock skew between nodes is a non-issue (same property the read filter has).
It orders nothing, so it sheds the clock; it is a heartbeat sequence, not a
logical clock.

## The liveness beacon

Each node broadcasts, once per anti-entropy tick, a small **ephemeral**
`GossipMessage::LivenessBeacon`:

```
LivenessBeacon { node_id, incarnation: u64, counter: u64 }
```

- `incarnation` = the node's **boot wall-clock time** (ms since epoch), read once
  at startup. A reboot naturally yields a later boot time → a higher incarnation,
  with **zero persisted state** (see restart, below).
- `counter` = a per-boot tick sequence, `+1` each beacon.

It is never merged into a book, never persisted, never relayed — it is authored
fresh each tick by the living origin about *itself*. Receivers keep an in-memory:

```
last_seen: BTreeMap<NodeId, (incarnation, counter, local_ms)>   // ephemeral
```

On receiving a beacon from `X`, refresh `last_seen[X]` iff it is **newer**:
`incarnation` is greater, or (`incarnation` equal and `counter` advanced). On a
newer beacon, stamp `local_ms = local_now()`. Every name and address entry
**owned by `X`** inherits `X`'s freshness — so the plane is **O(nodes), not
O(entries)**: one tiny beacon per node per tick refreshes all of that node's
records at once.

### Why this beats re-stamping the HLC (on every axis)

- **Zero flash churn.** The durable entry now persists only on a real field
  change, so `7bf` can freely take its *don't-bump-unless-changed* fix — the
  tension between `7bf` and this bead **dissolves** (we no longer force the
  persisted HLC to move).
- **No relay pollution — for free.** This was the whole reason the re-stamp
  variant existed. In the address book every node relays every peer's entry each
  tick, so "I received a message about X" does not mean X is alive. A beacon is
  authored *only by X about itself* and is ephemeral, so there is no durable copy
  for anyone to relay. If X is dead, X authors no beacon and nobody fabricates
  one → `last_seen[X]` ages out. No need for origin-only announce, and no need
  for immediate-sender plumbing (which the gossip layer does not expose today).
- **One rule, all lanes.** Liveness is per-node; services, names, and addr-book
  entries all key off the same `last_seen[owner]`.

### Restart (the one wrinkle, still zero-flash)

If X reboots and reset `counter` to 0, a receiver holding X's old `counter=500`
would reject `0` as "not newer" and think X dead until it climbs back past 500.
The `incarnation` handles this: X's post-reboot boot time is later than its
previous one, so the new beacon's `incarnation` is strictly greater and is
accepted regardless of `counter`. No incarnation is persisted — it is read from
the system clock at boot. (Edge: wall clock running backwards across a reboot is
rare and NTP-corrected; accepted. This `incarnation` field is also the natural
seam to the `4hl` SWIM upgrade, which uses incarnation numbers for
suspicion/refutation.)

## Staleness, read-side filter, hard-expiry

`STALE_THRESHOLD = K * ANTI_ENTROPY_INTERVAL`, `K = 3` (→ 60s: tolerates two
missed beacons before declaring stale). An owner `X`'s records are **stale** iff
`X` has no `last_seen` entry, or `local_now() − last_seen[X].local_ms >
STALE_THRESHOLD`. A record this node **originates** is always fresh (we beacon
ourselves; seed `last_seen[self]` at boot).

Stale records are **filtered on read, not deleted:**
- DNS `ServiceTable::lookup_a/exists/srv/txt` skip names whose owner is stale →
  the name NXDOMAINs (via the existing `exists`→false fall-through) instead of
  handing back a black-hole IP. **This is the headline fix.**
- `directory.json` / any `status` surface render stale records as `stale` rather
  than dropping them (operator visibility).
- the record **stays in the book.** If the owner returns, its next beacon
  refreshes `last_seen` → every name it owns silently un-stales. No tombstone, no
  re-claim.

A longer **hard-expiry** (`HARD_EXPIRY ≫ STALE_THRESHOLD`, e.g. 1h) finally
removes a still-stale record from the book so it cannot grow unbounded — the
addr-book "age out long-gone peers" and services "drop a never-returning owner"
requirements in one rule.

## Tombstone GC (bounded retention, resurrection-safe)

Retain an `unpublish` tombstone for `TOMBSTONE_RETENTION` from when this node
**first observed** it (a local `observed_at`, same ephemeral family), then drop
it. `TOMBSTONE_RETENTION` is generous (≫ STALE_THRESHOLD; sized to exceed the
longest partition worth surviving — start 1h, tunable).

**Why bounded GC is resurrection-safe** (the property the bead demands):

- A learned service entry is *never* re-announced by anyone but its owner
  (`announce_service_book_v2` broadcasts own entries only). Once the owner
  unpublishes, **nothing in the mesh re-announces the name.** A partitioned node
  holding a stale live copy serves it locally but never pushes it.
- While the tombstone lives, it rejects any owner replay older than the tombstone
  HLC (`apply_service_publish_v2` → `RejectedByTombstone`, already implemented) —
  in-flight stale packets can't revive it.
- After GC, resurrection would require some node to *re-announce* the dead name.
  None does. A partition still holding the live entry, on heal, does not push it,
  and its own read-side staleness filter has already stopped it resolving
  locally, with hard-expiry removing it.

So the tombstone only has to outlive the owner's own in-flight stale packets plus
tombstone propagation — both bounded. Keeping tombstones forever (today's
behavior) buys nothing the staleness filter doesn't already provide.

## Constants (one place, tunable)

```
ANTI_ENTROPY_INTERVAL = 20s     // existing — beacon rides this cadence
STALE_THRESHOLD       = 60s     // 3 intervals — stop resolving / mark stale
HARD_EXPIRY           = 1h      // drop from the book entirely
TOMBSTONE_RETENTION   = 1h      // then GC the tombstone
```

## Where it runs

- **Beacon emit:** one extra `sync.publish(LivenessBeacon{..})` at the top of the
  existing anti-entropy tick. No new timer, no flash write.
- **Beacon ingest:** a new arm in the gossip dispatch that updates the in-memory
  `last_seen` map (never persists).
- **Sweep:** a single pass at the tick — recompute stale flags from `last_seen`,
  hard-expire past `HARD_EXPIRY`, GC tombstones past `TOMBSTONE_RETENTION`. One
  cadence, one path.
- **Read filter:** a pure predicate on `last_seen`, independent of the sweep — DNS
  correctness never waits for the next tick.

## Acceptance (from the bead)

- ✅ a service/name whose owner has been offline for `STALE_THRESHOLD` stops
  resolving (no black-hole) and shows `stale` — read filter.
- ✅ tombstones GC'd after bounded retention without a stale re-announce
  resurrecting the name — GC argument.
- ✅ addr-book entries for long-gone peers age out — hard-expiry.
- ✅ a healing partition cannot resurrect a GC'd entry — GC argument.
- ✅ the same threshold/mechanism serves all three lanes — liveness is per-node;
  every lane keys off `last_seen[owner]`.

## Sharp edges

- **Forgery / withholding.** A malicious node could beacon "X is alive" or
  suppress X's. That is the signed-identity / web-of-trust problem (`e21.5`),
  explicitly out of scope; the re-stamp alternative had the identical exposure,
  so this is not a regression. Note and defer.
- **Direct-receipt is partition-blind.** X may be unreachable from A yet alive and
  heard by D; A will call X stale. Acceptable for the naming slice (a name simply
  NXDOMAINs — the safe failure) and it is exactly what `4hl` (SWIM, gossip the
  "who I heard from" digest) upgrades later. The beacon's `incarnation` field is
  forward-compatible with that work.
- **Clock skew is out of scope by construction:** liveness is receiver-local time
  deltas only; the beacon carries no wall clock.
- **Flapping owner** re-announcing right at the threshold edge flaps stale↔live;
  `K = 3` (two tolerated misses) gives margin. Not a correctness issue — a stale
  name NXDOMAINs.

## Why not …

- **Re-stamp the entry's HLC every tick (the first draft of this design).**
  Encodes liveness *into* the durable CRDT, forcing a flash write every tick
  (`7bf` churn) and constraining `7bf` to a weaker fix. The ephemeral beacon
  gets the same unified per-node signal with zero flash and no relay-pollution
  workaround. Rejected once the beacon was seen.
- **A gossiped GC watermark HLC.** Reintroduces a shared, gossiped CRDT purely to
  coordinate deletion — unnecessary once you see no node re-announces a dead
  name, so local bounded retention is already resurrection-safe.
- **Deleting stale entries immediately (no filter).** Loses silent recovery
  (owner returns → un-stale for free) and turns a transient miss into a
  re-converge. Filter-then-hard-expire keeps both.
- **Per-lane thresholds.** The lanes share a cadence and now a per-node signal;
  one threshold is the bead's "not per-lane ad-hockery" mandate.
- **Full SWIM now (`4hl`).** Partition-robust and the right long-term substrate,
  but a whole membership subsystem; overkill for the naming-staleness slice. The
  beacon is the forward-compatible floor it builds on. Deferred, thread kept.
