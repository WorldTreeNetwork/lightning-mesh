# Tasks

- [x] `cargo test -p mjolnir-mesh --lib iroh_dial` and `cargo test -p mjolnir-hello --bin mjolnir-hello iroh_dial` pass against this delta

## From advise 2026-11-26 (send-back)

Detail and code lines are in `reviews/2026-11-26-advise.md`.

- [x] R1: scope "Only an insert or an update SHALL rewrite the projection" to gossip receipt, and state that the sweep rewrites the projection every tick
- [x] R2: state the full receipt check (port, address, encodings, signature, the receiver's clock, then the book cap), and state that receipt cannot check the TCP peer or that the challenge was issued
- [x] R3: add the re-announce of the live book, and correct the THEN of "Restart does not revive a socket" so it allows a refill from peers
- [x] R4: word hello's cap by address instead of "a new endpoint id", and add the scenario where a known id moving onto a full address is 400 at hello
- [x] Add a test for a known id moving onto a full address in the book
- [x] Add a hello test for the cap that also shows the challenge is not spent
- [x] Re-advise the edited delta before act or fold
- [x] Fresh advise after preparation

## From advise r2 2026-11-26 (send-back)

R1 to R4 match the code except one sentence. Detail and the test body are in `reviews/2026-11-26-advise-r2.md`.

- [x] R5: bound "A claim whose endpoint id is already one of the live hints at that same address SHALL still be accepted" by the count. hello refuses a renewal when the address holds 9 or more live hints, and the book scenario in the same requirement produces 9
- [x] Add a test that a renewal is refused at hello when the address holds nine live hints

Findings, not owed by this change:

- With nine live hints at one address, every renewal from that address is refused at hello until one fades. Needs a bead, or a line on `mjolnir-mesh-23ks.5`.
- The restart scenario says "a peer's re-announced hint". Any accepted hint from a peer refills the book.

- `design.md` says other nodes are unaffected by an off clock. If the off node is the client's gateway, nothing is spooled or gossiped.
- `docs/deploy/mesh-app-publishing.md` says "within two minutes of the router". The code allows 90 s behind.
- hello writes the spool file in place, so a sweep can rename a half-written claim to `.json.bad` after the POST returned 200. Needs a bead.
- The book has no whole-book bound, only the per-address cap. Needs a bead or a stated bound.
- For `mjolnir-mesh-23ks.3`: confirm fleet nodes hold wall time with the uplink down.
