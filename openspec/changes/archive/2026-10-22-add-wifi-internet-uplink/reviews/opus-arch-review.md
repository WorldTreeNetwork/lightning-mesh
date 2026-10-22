# Architecture review — add-wifi-internet-uplink

> **ADVISE:** send-back
> **READER:** opus-arch-review
> **SPAWN:** /home/dorje/work/WorldTree/lightning-mesh/.spawns/mjolnir-mesh-bf7.2-1792653881-83222-ba87d2f9

Work node `mjolnir-mesh-bf7.2`. Read-only review, done 2026-10-22. I didn't
change any product code, banner, fold state or tasks.

## Blind take (written before opening design.md / tasks.md)

I wrote this from the proposal's Why, the living
`network-change-transactions` spec, and the cited code (`crates/mjolnir-apply`,
`deploy/openwrt/files/usr/sbin/mjolnir-apply`, `mjolnir-dongle`,
`crates/mjolnir-mesh/src/babel/config.rs`).

1. **Pin.** `OpenWrtAdapter::apply` (`crates/mjolnir-apply/src/openwrt.rs`)
   only copies UCI files. Nothing reloads wifi, netifd or meshd between
   `apply()` and `check_health()`, or after a restore. A station plan has to
   bring the radio up and restore it again inside the transaction. Without
   that, every upstream observation describes files, not the radio.
2. **Pin.** Today the adapter returns `Unknown` for every `required_health`
   other than `AppliedRevision`. `verify_restoration` returns `Err` if any of
   them is listed. So a station plan that requires upstream evidence always
   restores and then lands in **recovery-required**, which blocks further
   managed changes. The adapter needs real probes on the target, and
   restoration must not require upstream evidence.
3. **Pin.** On gateway nodes babeld redistributes the kernel `0.0.0.0/0`
   (`babel/config.rs:144`). A DHCP lease installs that default route. If the
   gateway role is on before upstream-bound DNS/HTTPS passes, a captive or
   dead upstream black-holes the whole mesh during the Verifying window.
4. **Pin.** A station on the backhaul radio has to follow the upstream AP's
   channel, which can drag or split 802.11s. `MeshReachability` must be
   required health. The candidate radio must not carry mesh-point, or the
   change must say why it may.
5. **Pin.** The candidate radio needs a stable identity (phy path), not
   `radioN`. `mjolnir-dongle` hotplug rewrites `wireless` outside the
   transaction lock.
6. **Pin.** The passphrase will be at rest in the snapshot, source and journal
   dirs. Receipts and observation details must be scrubbed.
7. **Refuse** exclusion by SSID string alone. Every mesh node's own client
   and guest BSSIDs must also be refused, because a station joining a sibling
   node makes a routing loop. The station must carry exactly one network.
8. **Refuse** upstream subnets that overlap `10.42/16`, `10.254/16` or the
   house ULA.
9. **Tradeoff.** Committing on local upstream evidence, with client internet
   left `Unknown`, versus requiring a `DownstreamClient` observation inside the
   120 s deadline (no client agent exists). I would take the first, labelled
   honestly, and keep any "online" UI tied to the second.
10. Wrong credentials must be told apart from slow DHCP or an out-of-range AP
    within the deadline. Otherwise the receipt detail gives the wrong cause.

## Comparison with design.md, tasks.md and the spec delta

### Where the author is ahead of my take, or matches it

- **Candidate pin (design 1).** Binding to node id + radio id + SSID, and
  treating a new BSSID for the same SSID as a new candidate, is stricter than
  my take. It covers my item 5's identity concern at the plan layer and
  defends against evil-twin reuse. Good.
- **Receipt pin (design 2).** This matches the living spec, and it lines up
  with what the engine already does. `sanitize_observations` downgrades
  non-client-vantage `ClientInternet` and `DownstreamClientForwarding` to
  `Unknown`. `is_qualifying_pass` requires a `DownstreamClient` vantage. So my
  item 9 tradeoff is already settled the honest way: commit is possible, and
  client internet stays unknown. Accept.
- **Second default route (design 3).** Naming AP3000's existing `0.0.0.0/0`
  export, and refusing a second route unless the export change is in the same
  revision-checked plan, is concrete and correct. It covers the multi-gateway
  half of my item 3.
- **Profile fit (design 6).** Refusing unless a committed profile already
  gives the radio the `station` role covers my item 4 *when a profile exists*.
- **Refusals (design 5).** Open-unselected, house client SSID, and no
  `br-lan` bridge all match items 7a and 7c.
- **Scope.** "Software builds the plan and the refusal rules", with no live
  association. This scoping is legitimate. Items 1, 5 (the hotplug race) and
  10 are live concerns and can wait for a follow-up. They don't block this
  change, as long as the change says so (see S1).

### What must be answered before build (send-back items)

Each item is a sentence or two in design.md, plus a spec scenario or task
where marked. None of them asks for a redesign.

**S1. Which observations does a station plan require, and what happens on
the real adapter today?**
The design never names the `required_health` set for a station plan. Against
the shipped `OpenWrtAdapter`, any upstream requirement comes back `Unknown`.
That triggers a restore, and then `verify_restoration` returns `Err`, which
gives **recovery-required**, not restored. So the change's own headline
scenario ("wrong passphrase → previous configuration remains in effect")
would end as recovery-required on hardware. The design must:
- (a) list the required observations: at least `UpstreamAssociation`,
  `UpstreamAddress`, `UpstreamRoute`, `UpstreamBoundDnsHttps`,
  `MeshReachability` and `ManagementReachability`.
- (b) say which subset restoration must prove. It should not include upstream
  evidence.
- (c) say plainly that the plan builder is tested against a fake adapter, and
  that running it through `OpenWrtAdapter` is refused or out of scope until a
  follow-up bead adds live activation (reload inside apply and restore) and
  real probes.

Without (c), a later worker can wire the builder into `mjolnir-apply`
`txn-plan.json`, and the first wrong passphrase on the fleet blocks the node.

**S2. "Stored by reference" vs. the transaction resource set.**
Design 4 says the passphrase is stored by reference. But the folded engine
only snapshots, restores and revisions four whole files (`wireless`,
`network`, `firewall`, `mjolnir`). OpenWrt `wifi-iface` reads `key` inline.
The design must say where the referenced secret lives, and pick one:
- **Outside the four files.** Restore then doesn't roll the secret back. A
  reused reference name can leave a restored config pointing at the new
  secret. The secret store also needs its own durable recovery, or the
  restore is incomplete. That conflicts with the living "durable recovery
  before destructive work" requirement.
- **Inside them.** The secret then sits in snapshot and source copies, which
  need a root-only mode and pruning. The revision is an **unkeyed blake3 over
  the file bytes** (`openwrt.rs` `revision()`), and it is returned to
  controllers in receipts and stale-preview refusals. With a known `wireless`
  layout and a WPA passphrase of 8+ characters, that hash is an offline
  dictionary oracle. "Receipt has no passphrase" passes while the receipt
  still leaks it.

Either choice is fine once it is stated. Add a task: a test that the revision
and receipt can't be used to confirm a guessed passphrase. Either the
revision is keyed, or the secret is excluded from the revisioned bytes and
has its own recovery path.

**S3. Gate the gateway export on upstream evidence, not on the lease.**
Design 3 covers the case where a gateway export already exists. It doesn't
cover the first-gateway case. Suppose the plan also turns on the meshd
`gateway` role (`mjolnir` UCI, `babel/config.rs:144`). Then babeld starts
announcing `0.0.0.0/0` the moment netifd installs the DHCP default route,
which is before `UpstreamBoundDnsHttps` is checked. A captive or dead
upstream then pulls every node's traffic for up to the deadline. The design
should say the mesh export goes live only after upstream-bound evidence
passes, or that a captive upstream restores before the export is
redistributed. A spec scenario would help ("lease + captive portal → no mesh
default route, restored").

**S4. Refuse the mesh's own networks by BSSID, not only the house client SSID.**
Design 5 refuses the configured house client SSID. It should also refuse
every SSID/BSSID served by a mesh node: the guest/captive SSID, a 2.4 GHz
`clientap2g` dongle AP, and any per-node SSID. A station on node A joined to
node B's guest AP creates a forwarding loop through the overlay, and SSID
matching alone misses a renamed or per-node AP. Also pin that the station
section carries exactly one network, with no fallback list. "Open SSID nobody
selected is not joined" only holds if the generated config can't contain a
second network.

**S5. A radio with no committed profile.**
Design 6 only covers a radio that already has a committed profile. When no
profile exists, nothing stops the plan from taking the radio that carries the
802.11s mesh-point. The design should either refuse a radio that carries
`mesh` mode in the current committed `wireless`, or require
`MeshReachability` (S1a) and say that a channel change on the backhaul is a
restore trigger.

### Lower priority (note in design.md; don't block)

- **N1.** The upstream DHCP subnet may overlap `10.42.0.0/16`, `10.254.0.0/16`
  or the identity-derived ULA. This should be a restore trigger (an
  `UpstreamRoute` failure) and not be NATed over.
- **N2.** `mjolnir-dongle apply` runs from USB hotplug without the
  `/etc/mjolnir/txn/lock` flock and rewrites `wireless`. If a station radio
  can be a dongle, a hotplug during a transaction changes the revision under
  the engine. That's live-path follow-up work; file a bead.
- **N3.** The receipt detail for wrong credentials should come from a
  supplicant reason (auth or 4-way handshake failure). A DHCP timeout reads
  the same at the transaction layer. Live-path follow-up.

## Steelman against my take

- My items 1 and 2 (live activation and probes) are fair to defer, because
  the change explicitly owes no live association. S1 doesn't ask the author
  to build them. It asks them to state the required evidence and to keep the
  builder off the real adapter until they exist. Without that statement, the
  spec scenario "previous committed configuration remains in effect" can't be
  met on the shipped adapter.
- Lane `lpv` owns radio choice, which might seem to cover S5. It doesn't:
  `lpv` picks the *best* receiver, while S5 is about refusing an *unsafe* one.
  The refusal belongs to this change's rule set.
- The secret question (S2) could be pushed to `ai0.7` (resident passphrase
  networks). But this change is the first to put a third-party secret into a
  revisioned, receipted file, so it sets the precedent `ai0.7` will inherit.

## Disposition

Send back for S1–S5. Once design.md answers them (and the spec gets the S2
and S3 scenarios), I expect this to be an accept. The pins on candidate
binding, receipt honesty and the AP3000 export are sound and shouldn't be
reworked.
