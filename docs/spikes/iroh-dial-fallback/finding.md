---
spike_id: iroh-dial-fallback
hypothesis_id: null
source_synthesis: null
run_at: 2026-11-26T14:44:49-08:00
time_spent_min: 35
verdict: no-go
version-pinned: iroh@1.0.0
---

# Spike: iroh 1.0 tries a supplied socket before lookup, and looks up only after a miss with no RelayUrl

## Claim Tested

iroh 1.0 tries a supplied 10.42 socket before address lookup, and calls address lookup when that socket misses and the address has no RelayUrl.

Pinned to crate `iroh` 1.0.0, checksum `6435544bb3a5c4e6ff7affaa0c0aa0d1bca45bd700226329d5059d3eb54f9dff` (`Cargo.lock`). Source is the crates.io checkout of that version. No synthesis file exists for this claim. The publishing sentence this claim backs is in `docs/deploy/mesh-app-publishing.md` under "Dialing another iroh endpoint on the mesh": "iroh tries the socket first. If it does not answer, address lookup supplies the relay."

## Minimal Test

Source read of the dial path, no production edit and no live dial. `Endpoint::connect` calls `resolve_remote`. That message is handled by `handle_msg_resolve_remote`, which inserts the supplied addresses and then decides whether the dial may proceed. `trigger_address_lookup` is the only start of address lookup on that path. The test checks three predicates: the supplied addresses are inserted and resolve completes before lookup is armed; a non-empty path set completes resolve immediately; the lookup gate does not mention `RelayUrl`. `docs/spikes/iroh-dial-fallback/repro-command.sh` re-checks those predicates against the installed crate.

## Evidence

`handle_msg_resolve_remote` (`remote_state.rs` 856–859) inserts the supplied addresses as `Source::App`, calls `paths.resolve_remote(tx)`, then calls `trigger_address_lookup`. `RemotePathState::resolve_remote` (`path_state.rs` 162–163) sends `Ok(())` immediately when `paths` is non-empty, so a supplied socket lets the dial proceed without waiting for lookup results. That is the first half of the claim.

`trigger_address_lookup` (`remote_state.rs` 866–879) returns only when `selected_path` is already set or a lookup stream is already running. It then calls `address_lookup.resolve`. The function body does not mention `RelayUrl`. On a fresh dial, `selected_path` is unset: the only writes in this file are a clear to `None` when the last connection closes (line 488) and `selected_path.replace` inside `select_path` (line 660), which runs after a path event, not when the address is inserted. A `RelayUrl` on the `EndpointAddr` becomes a path via `to_transports_addr` (line 1514) and still does not set `selected_path`, so it does not skip the lookup.

`Endpoint::connect` docs (`endpoint.rs` 1029–1030) say lookup is used when the remote is not reachable on the supplied direct addresses and there is no `RelayUrl`. That sentence is stricter than `trigger_address_lookup`. Lookup is armed in the same handler as the insert, before any miss can be observed, and it is armed whether or not a relay URL was supplied.

The predicate script printed `RESULT=predicates hold`. Excerpts are in `evidence.code`. The log is `evidence.log`, and the tracked copy is `evidence.txt` because `*.log` is gitignored.

## Verdict: no-go

The first half holds: a supplied socket is installed, and resolve completes before lookup results exist. The second half does not. Address lookup is not called because the socket missed, and it is not withheld because the address carries a `RelayUrl`. Both halves are one claim. The claim is false.

## Why the Hypothesis Is Wrong

The fallback described by the docs and by the publishing sentence is not the gate in `trigger_address_lookup`. The handler arms lookup immediately after inserting app-supplied paths, whenever nothing is selected yet. A miss is not an input to that function. A relay URL on the dial address is not an input to that function either. The direct socket can still be tried without waiting, but lookup is concurrent with that try, not a second step that starts only after the socket fails.

## Recommended Replan

Keep the signed dial record as shipped. Steer the client sentence before editing `docs/deploy/mesh-app-publishing.md`. The sentence that matches this source is: a supplied 10.42 socket is installed and the dial does not wait for address lookup; lookup is also started in that same step when no path is selected yet, including when the address has a `RelayUrl` and before the socket has failed. This spike does not choose that sentence. It does not change the postcard record, the router `RelayMode`, or the publishing doc.
