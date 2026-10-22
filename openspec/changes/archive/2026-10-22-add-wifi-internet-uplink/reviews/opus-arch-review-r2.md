# Architecture review r2 — add-wifi-internet-uplink

> **ADVISE:** accept
> **READER:** opus-arch-review
> **SPAWN:** /home/dorje/work/WorldTree/lightning-mesh/.spawns/mjolnir-mesh-bf7.2-1792654293-88970-a6a049ca

Work node `mjolnir-mesh-bf7.2`. Fresh read-only review, done 2026-10-22,
after the r1 send-back in `reviews/opus-arch-review.md`. I changed no product
code, banner or fold state. The only edit to `tasks.md` is the "Fresh advise"
checkbox.

## Blind take (written before opening design.md / tasks.md)

I wrote this from the proposal's Why, the living `network-change-transactions`
spec, `deploy/openwrt/setup-wireless.sh` and `crates/mjolnir-apply`.

1. **Pin.** The upstream AP sets the station radio's channel. If the station
   shares the backhaul radio, the fleet-wide mesh channel moves and the island
   splits. Refuse that radio, or model it explicitly. Also say what happens to
   a co-located client AP on the station radio.
2. **Pin.** Exclude our own networks by identity (BSSIDs from every node, plus
   our client and mesh SSIDs), not by SSID string. Join an open SSID only when
   the plan selected it.
3. **Pin.** OpenWrt reads the `key` inline from `wireless`, so the snapshot,
   journal and retained recovery artifacts all hold the passphrase.
   Redaction has to cover them, plus receipts, hello.mesh, logs, and the
   reused-ID plan comparison.
4. **Pin.** Wrong credentials, an AP that isn't visible, and a lease with no
   internet are different failures. Each needs a bound and each restores the
   previous committed plan, verified independently.
5. **Pin.** An upstream subnet that collides with `10.42/16`, `10.254/16` or
   `192.168.1.1` recovery is refused, or restores once the lease shows it.
6. **Pin.** Client internet stays unknown unless a downstream client-path
   observation passes through the forward/NAT path. A ping from the station
   doesn't count.
7. **Pin.** Say how the mesh learns the default route (babel redistributes
   `0/0`) and how restore withdraws it.
8. **Refuse** auto-join, automatic receiver selection (`lpv`) and live fleet
   writes in this change.
9. **Refuse** station credentials edited outside a transaction.
10. **Tradeoff.** Commit on local health with client internet left unknown,
    or block commit until a real downstream client proves internet. The
    second is honest but may never commit in a house with no wired client.
    I would take the first, labelled honestly.

## Comparison with r1 and the revised design

My blind take mostly overlaps r1. The new points are item 1's co-located AP
and item 9. Neither blocks (see N4 and N5).

| r1 item | Where it's answered | Verdict |
|---|---|---|
| **S1** required health, restoration subset, adapter | design "Required health": six observations listed. Restoration proves only `ManagementReachability` + `AppliedRevision`. Builder is tested on a fake adapter and refused on `OpenWrtAdapter` until live probes and reload exist. Tasks 4 and 6 carry it. | Answered |
| **S2** where the secret lives | design "Secret file": `/etc/mjolnir/secrets/uplink/<candidate-id>`, mode 0600. It is outside the four revisioned files and has its own root-only journal sidecar snapshot. Restore puts the old secret back before the result reads restored. The spec forbids the bytes or an unkeyed hash in revisions and receipts. A guessed-passphrase test is in task 6. | Answered |
| **S3** first-gateway export | design "First gateway": the export is installed only after `UpstreamBoundDnsHttps` passes. A captive lease installs nothing and restores. Spec scenario "Lease on a captive upstream" added. | Answered |
| **S4** mesh BSSIDs, single network | design "Which networks": every BSSID a mesh node advertises is refused (house, guest, captive, dongle AP, per-node SSID). Exactly one network, no fallback list. Spec scenario added. | Answered |
| **S5** radio with no profile | Mesh-mode radios are refused "whether or not a profile exists". Spec scenario added. | Answered |

r1's N1 (subnet overlap) is now a restore trigger in the design. N2 and N3
(dongle hotplug vs. the flock, supplicant reason strings) are listed as
"Not owed here". That matches what r1 asked for.

## Steelman against my take

- Item 3 (the secret inline in `wireless`) looked like it broke S2. It doesn't
  at the plan layer, because the design keeps the secret out of the four
  files. How the secret reaches the supplicant is live-path work, which this
  change explicitly doesn't owe. N2 records the constraint the follow-up
  inherits.
- Items 4 and 6 are covered by the living spec and the engine. Restore and
  independent verification are already there. `sanitize_observations` and
  `is_qualifying_pass` already refuse non-client vantage for client internet.
  Restoring without upstream evidence (S1) closes the "wrong cause, wrong
  state" half. Telling wrong credentials apart from out-of-range is deferred,
  and that's honest.
- Item 10 was settled the honest way by the existing engine and design pin 2.

## Build-time notes (not blocking; the builder should honour them)

- **N1. The restoration subset needs a home in the contract.** `Plan` has one
  `required_health` list (`model.rs:44`). `OpenWrtAdapter::verify_restoration`
  walks that same list (`openwrt.rs:167`). "Restoration proves only
  management + revision" therefore needs a field or a fixed rule that the
  engine and every adapter honour. Putting it only in the fake adapter would
  make the task-6 test vacuous. Refusal on `OpenWrtAdapter` should also
  happen on the target before mutation: reject a plan whose required health
  the adapter can't observe. A builder that simply doesn't call the adapter is
  not enough, because a hand-written `txn-plan.json` still lands in
  recovery-required today.
- **N2. `plan_digest` is also an oracle.** `validate_plan` hashes the
  serialized plan with unkeyed blake3 (`engine.rs:694`). The result is echoed
  in receipts and reused-ID refusals (`model.rs:76,173,187`). If the
  passphrase rides inside `Plan`, the spec's "no unkeyed hash in receipts"
  rule is broken, and so is the plan file under the txn dir. The plan should
  carry only a secret reference. The guessed-passphrase test should cover
  `plan_digest` and the stale-preview refusal, not only `revision`. The
  follow-up that feeds the supplicant must not write `key` back into
  `/etc/config/wireless`, or S2 falls apart.
- **N3. Revisions must still change when only the passphrase changes.** The
  revision excludes the secret bytes. So a passphrase-only edit, or a
  concurrent secret write, leaves `expected_revision` unchanged and goes
  unseen. Fold a keyed digest or a monotonic secret generation into the
  revision. The spec allows a keyed hash.
- **N4. The mesh-BSSID set needs a fail-closed source.** Name where the set of
  node-advertised BSSIDs comes from (radio inventory / address book). If that
  set isn't current, refuse rather than plan. The subnet-overlap restore
  trigger already catches a sibling node's lease in `10.42/16` after the
  fact, which is good defence in depth.
- **N5. Co-located AP on the station radio.** On a two-radio node with one
  mesh radio refused, the station lands on the client AP radio. That AP then
  follows the upstream channel, so its clients re-associate and the band is
  fixed by the radio. Record this as an expected effect, not a failure.
  Refusing a mesh *interface* is the precise rule: a radio "carries a
  mesh-point `wifi-iface`" rather than "is mesh mode". Tighten that wording
  at fold time.
- **N6. The export gate is a second mutation in the transaction.** "Export
  only after `UpstreamBoundDnsHttps` passes" means either a mid-Verifying step
  or a meshd-side condition. The station's DHCP route also must not land
  where babeld redistributes it (`babel/config.rs:144`) before then. Either
  way, restore must withdraw it. That belongs to the live follow-up, but the
  fake-adapter test should model the order.

## Disposition

Accept. S1–S5 are answered in design.md, the spec delta and tasks.md. The
pins on candidate binding, receipt honesty, the AP3000 export and the refusals
are sound. N1–N3 are the notes I'd most want the builder to read before
writing tests.
