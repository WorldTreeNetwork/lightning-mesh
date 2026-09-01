# Project context

Lightning Mesh — a decentralized OpenWrt router mesh. The L3 overlay
(iroh + babeld + CRDT) is the product; the radio is plumbing.

This file is conventions. It is not a requirements store. Requirements live
in `openspec/specs/` (what is built) and `openspec/changes/` (what should
change). Reasoning that is not a requirement lives in `docs/` and
`ARCHITECTURE.md`, and must name a change-id when it implies work.

Issue tracking is **bd** (beads). Beads are the work graph. OpenSpec is
what is true and what should change. `bd ready` is not the OpenSpec
ready-set.

## Where work lands

| Kind of work | Lands in |
|---|---|
| New or changed behavior | `openspec/changes/<verb-led-id>/` |
| Restore intended behavior, typo, pin, comment, test for existing spec | Direct fix. No change. |
| Why the system is shaped this way | `ARCHITECTURE.md` (amend, do not delete) |
| Hard-won fact | `docs/LEARNINGS.md` |
| Work-graph state | beads (`.beads/issues.jsonl`) |

A change is the right landing zone when you can write a `#### Scenario:` that
fails today and passes after.

## Definition of done

A change is done when all of these hold:

1. Its living-spec deltas have been **folded** into `openspec/specs/`.
2. The change directory has been **archived** to
   `openspec/changes/archive/YYYY-MM-DD-<id>/`.
3. Every checkbox is work **this** change owed. Handoffs and out-of-scope
   are bullets.
4. The proposal has `## User journey & surfaces`, or an explicit
   `No new UI because <reason>` naming the surface people actually use.

Code landing without fold+archive is not done for a change. A vibe/direct
fix is done when the commit is on `main`.

## Disposition banners

The first non-empty line of `proposal.md` after the title heading is a
banner. Status lives in the file because retrieval strips paths.

```
> **PENDING**
> **ACTIVE BUILD**
> **PARKED** — revive when <condition>
```

Agents draft PENDING. Humans replace it with ACTIVE BUILD (or you are
reading an activation in chat). PARKED is not available work.
