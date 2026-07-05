# Calvin / FaunaDB deterministic ordering — longshot notes

> **Status:** parked / speculative (2026-07-04). Not a plan, not a bead. A record of
> a research pass on whether Calvin's consistency method helps our HLC + flash-churn
> work. Verdict: **don't adopt the mechanism; one sub-idea worth shelving.**

## The question

FaunaDB scored well on Jepsen and — unlike Spanner — needs no clock synchronization.
We've been wrestling with HLCs and with *not* rewriting router flash constantly
(`7bf` churn, resolved for liveness by the e21.9 ephemeral beacon). Does Calvin's
trick help either problem?

## What Calvin actually does

Calvin (Thomson & Abadi, Yale; the protocol under FaunaDB) is a **deterministic**
transaction layer:

1. A **sequencing layer** batches transactions in ~10ms epochs into a **globally
   replicated append-only log** (Paxos/Raft). The log position *is* the total order.
2. Every replica reads its local copy of that agreed log and executes
   **deterministically** in that exact order. Same input order + deterministic
   execution → identical state everywhere, with **no 2PC, no cross-node locks**.

**Why no clock sync:** order comes from *consensus on a log*, not from timestamps.
That's the deliberate contrast with Spanner, which needs TrueTime (GPS/atomic clocks
+ commit-wait) to timestamp-order. Calvin says "let a machine agree the order instead
of trusting physics." Jepsen liked it because strict serializability falls out of a
single agreed order every replica replays identically — no clock-skew anomaly surface.
(Jepsen still found ~19 issues and that Fauna's *docs* overclaimed strict-SR where you
sometimes got snapshot isolation — the model held, the marketing didn't.)

## Why it does NOT fit this mesh

Calvin assumes exactly the three things this project rejects by design:

| Calvin needs | Our mesh is |
|---|---|
| A **global Paxos/Raft log** every write passes through before it's visible | Symmetric, **non-authoritative** nodes; no coordination point |
| A **quorum** to progress → **CP**: minority side of a partition *blocks* | **AP/CRDT**: every node writes locally; partition is the *normal* state |
| Relatively **stable, known membership** | Churn-forever membership |

Adopting Calvin's mechanism means reintroducing the consensus log + authority we
removed on purpose, and it would **freeze the minority side of every partition** — a
non-starter for a radio mesh. So: not wholesale.

## What already transfers (we did it independently)

Calvin's real lesson — *"derive order from an agreed sequence, not from wall clocks"* —
we've already applied:

- The **e21.9 liveness beacon** carries `incarnation + counter` and **no wall clock**;
  staleness is a **receiver-local** delta (`local_now − last_seen`). Same instinct as
  Calvin-vs-Spanner, in miniature.
- The **HLC counter** is the CRDT-world partial-order version.

The difference is intentional CAP positioning: Calvin buys a **total** order by paying
for consensus; we accept a **partial** order (CRDT + FWW-on-HLC tiebreak) *because we
refuse to pay that price*. Not a gap Calvin patches — a different corner of CAP.

## The one shelf-worthy idea (flash, not consistency)

Calvin's structural split is **"the log is truth; materialized state is a derived,
throwaway replay."** That maps onto flash churn better than onto consistency:

- Today we rewrite a materialized `.state` blob on every real change. The
  Calvin-flavored alternative: **append-only intent log + periodic snapshot/compaction**.
  Sequential appends are kinder to flash than rewrite-in-place; state becomes a
  boot-time replay of `snapshot + tail-of-log`.
- **Caveat:** an append-per-write log can *increase* total bytes written unless writes
  are batched/coalesced and snapshots are rare. And e21.9 already killed the dominant
  churn source (the every-20s HLC restamp), so this is a "if *data-write* churn ever
  bites" idea — not urgent. Revisit only if durable CRDT writes (not liveness) become
  the flash bottleneck.

## Verdict

- **Consistency mechanism:** reject — CP consensus-log system, needs authority + quorum
  + stable membership, blocks under partition. Opposite of our thesis.
- **The cited insight** ("order without clock sync"): real, and already internalized
  (beacon is clock-free by construction; HLC counter is the partial-order equivalent).
- **Keep on the shelf:** Calvin's *log-is-truth, state-is-derived* separation as a
  flash-write pattern (append + snapshot vs. rewrite), if data-write churn becomes the
  problem.

## Sources

- Jepsen: FaunaDB 2.5.4 — https://jepsen.io/analyses/faunadb-2.5.4
- Calvin: Fast Distributed Transactions (Thomson & Abadi, TODS'14) — http://www.cs.umd.edu/~abadi/papers/calvin-tods14.pdf
- Spanner vs. Calvin (Fauna) — https://fauna.com/blog/distributed-consistency-at-scale-spanner-vs-calvin
- Calvin — The morning paper — https://blog.acolyer.org/2019/03/29/calvin-fast-distributed-transactions-for-partitioned-database-systems/
- Calvin, the Magic of Determinism — https://www.mydistributed.systems/2020/08/calvin.html

## Related

- `docs/network-coordination/lane-staleness.md` (e21.9 — the ephemeral beacon)
- `docs/network-coordination/gossip-and-crdt.md` (HLC, FWW merge rule)
- bead `7bf` (flash-churn), `4hl` (SWIM upgrade the beacon seams into)
