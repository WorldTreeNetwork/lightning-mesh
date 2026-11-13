# mjolnir-node: Full-Featured Standalone Node

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Elevate `mjolnir-node` from a gossip-only observer to a full-featured standalone node that participates in mesh state (CRDT apply, anti-entropy, liveness, DNS) while remaining platform-agnostic (macOS/Linux, no TUN/Babel).

**Architecture:** `mjolnir-node` stays a single binary that reuses the `mjolnir-mesh` library for all CRDT types, merge functions, and the `GossipTransport` trait. The node owns its own in-memory CRDT books and a spool directory for persistence. It joins the gossip topic, applies incoming messages to local state, broadcasts anti-entropy snapshots, sends liveness beacons, and serves `.mesh` DNS queries. TUN tunnels, Babel routing, DHCP, and subnet claiming remain in `mjolnir-meshd` (Linux-only).

**Tech Stack:** Rust 2024, iroh (same version as mjolnir-mesh), postcard, clap, tokio, tempfile, serde.

---

## Task 0: Audit current state and verify build

**Objective:** Confirm current `mjolnir-node` compiles and all tests pass, establishing a clean baseline.

**Files:**
- Read: `crates/mjolnir-node/Cargo.toml`
- Read: `crates/mjolnir-node/src/main.rs`

**Steps:**
1. Run: `cargo build -p mjolnir-node`
2. Run: `cargo test -p mjolnir-node`
3. Run: `cargo clippy -p mjolnir-node`
4. Confirm 14 tests pass, build succeeds.

**Verification:** Build and tests clean. Document any warnings.

---

## Task 1: Add CRDT book types as dev-dependencies in mjolnir-mesh

**Objective:** Ensure `mjolnir-mesh` exports all CRDT book types that `mjolnir-node` needs (ServiceBookV2, LeasedNameBook, CoordinateBook, NodeNameBook, UserBook, AddrBook, LeaseBook, ServiceTombstoneBook).

**Files:**
- Modify: `crates/mjolnir-mesh/src/lib.rs` — add book types to `pub use crdt::...` exports

**Steps:**
1. Read `crates/mjolnir-mesh/src/lib.rs` (current state).
2. Identify which book types are already exported and which are missing.
3. Add missing book type exports.
4. Verify: `cargo build -p mjolnir-mesh --features daemon`

**Key types needed:**
- `LeaseBook` (already exported)
- `ServiceBookV2` (already exported)
- `ServiceTombstoneBook` (already exported)
- `LeasedNameBook` (NOT exported — needs adding)
- `CoordinateBook` (already exported)
- `NodeNameBook` (already exported)
- `UserBook` (already exported)
- `AddrBook` (already exported)

**Verification:** `cargo build -p mjolnir-mesh --features daemon` succeeds.

---

## Task 2: Add spool directory for persistence

**Objective:** Add a `--spool` CLI option and a spool directory manager that persists CRDT books to disk and reloads them on startup.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — add `--spool` CLI arg, implement spool load/save

**Steps:**
1. Add `--spool <path>` to the `Cli` struct.
2. Implement `Spool` struct with methods:
   - `new(path) -> Self` — create/open directory
   - `save_book<T: Serialize>(&self, name: &str, data: &T) -> Result<()>` — write JSON to `<path>/<name>.json.tmp`, then rename
   - `load_book<T: for<'de> Deserialize<'de>>(&self, name: &str) -> Result<Option<T>>` — read JSON
3. On startup, load all books from spool.
4. The spool uses atomic tmp+rename for crash safety.

**Code sketch:**
```rust
struct Spool {
    path: PathBuf,
}

impl Spool {
    pub fn new(path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&path).context("create spool dir")?;
        Ok(Self { path })
    }

    pub fn save(&self, name: &str, data: &impl serde::Serialize) -> Result<()> {
        let tmp = self.path.join(format!("{name}.json.tmp"));
        let final_ = self.path.join(format!("{name}.json"));
        let json = serde_json::to_string(data)?;
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &final_)?;
        Ok(())
    }

    pub fn load<T: for<'de> serde::Deserialize<'de>>(&self, name: &str) -> Result<Option<T>> {
        let path = self.path.join(format!("{name}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path)?;
        Ok(Some(serde_json::from_str(&json)?))
    }
}
```

**Verification:** `cargo build -p mjolnir-node` succeeds.

---

## Task 3: Add CRDT apply handler

**Objective:** Replace the print-only message handler with a full apply loop that merges incoming gossip messages into local CRDT books.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — add CRDT apply handler

**Steps:**
1. Create `CrdtStore` struct holding all book types:
```rust
struct CrdtStore {
    leases: LeaseBook,
    services_v2: ServiceBookV2,
    services_tombstones: ServiceTombstoneBook,
    leased_names: LeasedNameBook,
    coordinates: CoordinateBook,
    node_names: NodeNameBook,
    users: UserBook,
    peer_addrs: AddrBook,
    subnet_claims: SubnetClaimMap,  // BTreeMap<Cidr, SubnetClaim>
}
```

2. Implement `apply(&mut self, msg: GossipMessage) -> MergeResult` for each variant:
   - `LeaseUpdate` -> `merge_lease`
   - `ServicePublishV2` -> `merge_service_v2`
   - `ServiceUnpublishV2` -> handle tombstone
   - `LeasedNamePublish` -> `merge_leased_name`
   - `CoordinateAnnounce` -> `merge_coordinate`
   - `NodeNameAnnounce` -> `merge_node_name`
   - `UserUpdate` -> `merge_user`
   - `PeerAddrUpdate` -> `merge_peer_addr`
   - `SubnetClaimUpdate` -> `merge_subnet_claim`

3. The handler signature changes from `FnMut(GossipMessage)` to `FnMut(GossipMessage, &mut CrdtStore)`.

**Verification:** All existing tests still pass. The apply function is pure merge dispatch.

---

## Task 4: Add anti-entropy broadcast

**Objective:** Periodically broadcast full state snapshots of all CRDT books to peers, so late joiners converge.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — add anti-entropy loop

**Steps:**
1. Add `--anti-entropy-interval <seconds>` CLI arg (default: 60).
2. Spawn a periodic task that:
   - Iterates all books in `CrdtStore`
   - Encodes each entry as a `GossipMessage`
   - Broadcasts via `sync.publish(msg)`
3. Mirror the daemon's anti-entropy pattern: broadcast subnet claims, services, leased names, coordinates, node names, peer addrs, users.
4. The anti-entropy loop runs on a tokio interval.

**Code sketch:**
```rust
async fn anti_entropy_loop(store: &CrdtStore, sync: &GossipSync<IrohGossipTransport>, interval: Duration) -> Result<()> {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        // Broadcast all lease entries
        for (mac, lease) in store.leases.iter() {
            let msg = GossipMessage::LeaseUpdate(lease.clone());
            if let Err(e) = sync.publish(msg).await {
                warn!(error = %e, "anti-entropy lease broadcast failed");
            }
        }
        // ... similar for services, leased names, coordinates, etc.
        info!("anti-entropy sweep complete");
    }
}
```

**Verification:** `cargo build -p mjolnir-node` succeeds.

---

## Task 5: Add liveness beacon broadcast

**Objective:** Each node broadcasts a liveness beacon on the anti-entropy cadence so peers can track it.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — add beacon in anti-entropy loop

**Steps:**
1. Track beacon counter and incarnation (boot wall-clock ms) in the node state.
2. Each anti-entropy tick, broadcast:
```rust
let msg = GossipMessage::LivenessBeacon {
    node_id: endpoint.id().to_string(),
    incarnation: boot_time_ms,
    counter: tick_count,
    egress: None,  // node is not a gateway
};
sync.publish(msg).await?;
```
3. On receive, update the local `LivenessTracker` (in-memory only, not persisted).

**Verification:** Beacon appears in gossip log.

---

## Task 6: Add `.mesh` DNS responder

**Objective:** The node serves `.mesh` DNS queries on localhost:5335, resolving from local CRDT books.

**Files:**
- Modify: `crates/mjolnir-node/Cargo.toml` — add `simple-dns` dependency (already in mjolnir-mesh)
- Modify: `crates/mjolnir-node/src/main.rs` — add DNS responder

**Steps:**
1. The `dns_responder` module is behind `#[cfg(feature = "daemon")]` in `mjolnir-mesh`. We need to either:
   - Option A: Export `dns_responder` under a separate feature flag (e.g., `dns`) in `mjolnir-mesh`.
   - Option B: Vendor the `dns_responder.rs` file into `mjolnir-node`.
2. Choose Option A: Add a `dns` feature to `mjolnir-mesh` that gates `dns_responder` alongside `daemon`.
3. Patch `mjolnir-mesh/Cargo.toml`: change `#[cfg(feature = "daemon")]` to `#[cfg(any(feature = "daemon", feature = "dns"))]`.
4. In `mjolnir-node`, add `mjolnir-mesh = { path = "../mjolnir-mesh", features = ["dns"] }`.
5. Start the DNS responder with a `CompositeTable` stacking `WellKnownTable` + `ServiceTable`.

**Verification:** `dig @127.0.0.1 -p 5335 hello.mesh A` returns the gateway IP.

---

## Task 7: Add Control API

**Objective:** A local HTTP API for publishing/unpublishing services, announcing node names, and querying state.

**Files:**
- Create: `crates/mjolnir-node/src/control.rs` — Control API server
- Modify: `crates/mjolnir-node/src/main.rs` — start control API

**Steps:**
1. Add `axum` dependency to `mjolnir-node/Cargo.toml`.
2. Implement handlers:
   - `POST /v0/publish` — publish a service (reads JSON body, calls `publish_service_v2`, broadcasts via sync)
   - `POST /v0/unpublish` — unpublish a service
   - `POST /v0/node-name` — announce a node name
   - `GET /v0/state` — dump current CRDT store as JSON
   - `GET /v0/peers` — list connected gossip peers
3. Bind to `127.0.0.1:5380` (same port as meshd).
4. The publish/unpublish handlers serialize a `GossipMessage::ServicePublishV2` and broadcast via `sync.publish()`.

**Code sketch:**
```rust
#[derive(Deserialize)]
struct PublishRequest {
    name: String,
    port: Option<u16>,
    ip: IpAddr,
    txt: Option<BTreeMap<String, String>>,
}

async fn publish(
    State(store): State<Arc<Mutex<CrdtStore>>>,
    State(sync): State<Arc<GossipSync<IrohGossipTransport>>>,
    Json(req): Json<PublishRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut s = store.lock().unwrap();
    let outcome = publish_service_v2(&mut s.services_v2, ...);
    // broadcast
    drop(s);
    Json(serde_json::json!({ "status": "ok" })).into()
}
```

**Verification:** `curl -X POST http://127.0.0.1:5380/v0/publish -d '{"name":"wiki","port":8080}'` works.

---

## Task 8: Add liveness tracker and stale-name filtering

**Objective:** Track peer liveness and filter DNS responses for stale owners.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — wire `LivenessTracker` into DNS responder

**Steps:**
1. Create `LivenessTracker` with 60s stale / 1h hard-expiry (matching daemon defaults).
2. Feed liveness beacons into the tracker in the message handler.
3. Pass the tracker to `ServiceTable::with_liveness()` when building the DNS `CompositeTable`.
4. Stale owners' names return NXDOMAIN from DNS.

**Verification:** After a node stops beaconing, its names stop resolving after 60s.

---

## Task 9: Add roaming support

**Objective:** Support a node that changes network location (e.g., laptop moving between Wi-Fi networks) by re-announcing its address book entry.

**Files:**
- Read: `crates/mjolnir-mesh/src/roam.rs` — understand existing roaming logic
- Modify: `crates/mjolnir-node/src/main.rs` — integrate roaming

**Steps:**
1. Read `roam.rs` to understand `RoamBook`, `RoamEntry`, and the roaming protocol.
2. If `roam.rs` is library-exported, import it. If not, the roaming logic is minimal and can be added to the node.
3. On each anti-entropy tick, also broadcast roaming entries if the node's IP addresses changed.

**Verification:** Node re-announces after IP change.

---

## Task 10: Add tests

**Objective:** Add thorough tests for all new functionality.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — add tests module
- Modify: `crates/mjolnir-node/Cargo.toml` — add `serde_json` dev-dependency

**Test targets:**
1. `spool_save_load_roundtrip` — write JSON, read back, verify equality
2. `spool_atomic_rename` — tmp file exists during write, final file appears after rename
3. `crdt_apply_lease_update` — verify merge_lease called on LeaseUpdate
4. `crdt_apply_service_publish_v2` — verify service inserted into book
5. `crdt_apply_coordinate_announce` — verify coordinate merged
6. `dns_resolved_well_known_name` — hello.mesh returns gateway IP
7. `dns_nxdomain_unknown_name` — unknown.mesh returns NXDOMAIN
8. `dns_service_from_book` — published service resolves via DNS
9. `liveness_stale_owner_nxdomain` — stale owner's service returns NXDOMAIN
10. `anti_entropy_broadcasts_all_books` — verify all book entries are encoded as gossip messages

**Verification:** `cargo test -p mjolnir-node` passes all tests.

---

## Task 11: Update documentation and commit

**Objective:** Update the binary's doc comments and help text to reflect the new capabilities.

**Files:**
- Modify: `crates/mjolnir-node/src/main.rs` — update module doc comments

**Steps:**
1. Update module doc: "This binary joins the mesh gossip topic, applies CRDT state, broadcasts anti-entropy, and serves `.mesh` DNS."
2. Update help text to list new subcommands/flags.
3. Run `cargo clippy -p mjolnir-node` — clean.
4. Run `cargo test -p mjolnir-node` — all pass.
5. Commit with message: `feat(mjolnir-node): full-featured standalone node with CRDT apply, anti-entropy, DNS, and control API`
6. Push.

---

## Files Likely to Change

| File | Action | Description |
|------|--------|-------------|
| `crates/mjolnir-mesh/src/lib.rs` | Modify | Export `LeasedNameBook` if missing |
| `crates/mjolnir-mesh/Cargo.toml` | Modify | Add `dns` feature flag for `dns_responder` |
| `crates/mjolnir-node/Cargo.toml` | Modify | Add `simple-dns`, `axum`, `serde_json` deps; add `mjolnir-mesh` with `dns` feature |
| `crates/mjolnir-node/src/main.rs` | Major refactor | CRDT store, spool, anti-entropy, liveness, DNS, control API, tests |
| `crates/mjolnir-node/src/control.rs` | Create | Control API HTTP server |

---

## Risks and Trade-offs

1. **dns_responder feature flag:** The `dns_responder` module is gated behind `daemon` in `mjolnir-mesh`. Adding a `dns` feature is the cleanest approach, but it means the daemon now has two features that both pull in DNS. This is acceptable — the daemon's `daemon` feature already includes `simple-dns`.

2. **Binary size:** Adding `axum` and `serde_json` increases compile time and binary size. This is acceptable for a full-featured node.

3. **CRDT book serialization:** We use JSON for spool persistence. This is simpler than postcard and sufficient for the node's use case (not performance-critical). Postcard could be added later.

4. **No TUN/Babel:** By design, `mjolnir-node` does NOT run TUN tunnels or Babel routing. These remain in `mjolnir-meshd` (Linux-only). The node is a stateful gossip participant, not a full router.

5. **Control API security:** The control API binds to localhost only. No authentication. This is acceptable for a development/management tool on trusted hosts.

---

## Execution Order

Tasks 0-5 (CRDT apply, anti-entropy, liveness) are the core. Tasks 6-9 (DNS, control API, roaming) are the value-add. Task 10-11 are verification.

**Critical path:** Task 0 (baseline) → Task 1 (exports) → Task 3 (CRDT apply) → Task 4 (anti-entropy) → Task 5 (liveness) → Task 6 (DNS) → Task 7 (control API) → Task 8 (liveness filtering) → Task 9 (roaming) → Task 10 (tests) → Task 11 (docs/commit).

---

## Verification Commands

```bash
# Build
cargo build -p mjolnir-node

# Test
cargo test -p mjolnir-node

# Clippy
cargo clippy -p mjolnir-node

# Workspace build
cargo build --workspace

# Workspace test
cargo test --workspace --lib
```
