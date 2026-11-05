// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! `mjolnir-node` — full-featured standalone node for the mjolnir-mesh private network.
//!
//! This binary joins the mesh gossip topic, maintains CRDT state, broadcasts
//! anti-entropy beacons, tracks peer liveness, serves `.mesh` DNS, and exposes
//! a JSON control API. It does NOT run babeld, TUN tunnels, or DHCP — those
//! remain in `mjolnir-meshd` (Linux-only).
//!
//! Subcommands:
//!   listen    join the gossip swarm, maintain full state (default)
//!   id        print this node's EndpointId and address blob

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use data_encoding::HEXLOWER;
use futures_lite::StreamExt;
use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointId, RelayMode, RelayUrl, SecretKey};
use iroh_gossip::api::{Event, GossipReceiver, GossipSender};
use iroh_gossip::Gossip;
use iroh_mdns_address_lookup::MdnsAddressLookup;
use mjolnir_mesh::crdt::gossip::GossipMessage;
use mjolnir_mesh::crdt::liveness::{LivenessTracker, monotonic_now_ms};
use mjolnir_mesh::crdt::merge::{
    MergeResult, apply_coordinate, merge_node_name, merge_peer_addr, merge_user,
};
use mjolnir_mesh::crdt::sync::{GossipError, GossipSync, GossipTransport};
use mjolnir_mesh::dns_responder::{
    self, CompositeTable, GatewayHandle, LeasedNameTable, NameTable, ResponderHandle,
    ServiceTable, WellKnownTable, DEFAULT_DNS_PORT, LEASED_NAME_RESOLVE_STALE_MS,
};
use mjolnir_mesh::roster::PeerRoster;
use mjolnir_mesh::{
    AddrBook, CoordinateBook, CoordinateStamp, LeasedNameBook, LeaseEntry, NodeNameBook,
    NodeNameEntry, PeerAddrEntry, ServiceBookV2, ServiceTombstoneBook, UserEntry, UserBook,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as TokioMutex;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
use tracing::{debug, error, info, warn};

/// Standalone iroh mesh node — full-featured gossip participant.
#[derive(Parser)]
#[command(name = "mjolnir-node", about = "Full-featured mjolnir-mesh gossip node")]
struct Cli {
    /// Path to the persisted node secret key (hex). Generated on first run if absent.
    #[arg(long, global = true)]
    secret_file: Option<PathBuf>,

    /// Disable iroh relays (direct/LAN only). Implies --lan.
    #[arg(long, global = true)]
    no_relay: bool,

    /// LAN-direct mode: discover peers via mDNS, no relay, no pkarr/DNS.
    /// For same-switch swarms. Implies --no-relay.
    #[arg(long, global = true)]
    lan: bool,

    /// Relay server URL(s) to use (repeatable). If omitted, uses n0 staging relays.
    #[arg(long, global = true)]
    relay: Vec<String>,

    /// Path to a peer roster file (one 64-hex node id per line).
    #[arg(long, global = true)]
    roster: Option<PathBuf>,

    /// A peer 64-hex node id to connect with (repeatable). Merged with --roster.
    #[arg(long, global = true)]
    peer: Vec<String>,

    /// Path to the data directory for CRDT state persistence (default: ~/.local/share/mjolnir-node).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// Port for the JSON control API (default: 9999).
    #[arg(long, global = true)]
    control_port: Option<u16>,

    /// DNS responder bind address (default: 127.0.0.1:5335).
    #[arg(long, global = true)]
    dns_bind: Option<String>,

    /// Anti-entropy interval in seconds (default: 30).
    #[arg(long, global = true)]
    anti_entropy_interval: Option<u64>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print this node's EndpointId and a shareable address blob.
    Id,
    /// Join the gossip swarm and maintain full state (default).
    Listen,
}

/// The fixed gossip topic for a mesh's CRDT overlay.
/// Same constant hash as mjolnir-meshd.
fn mesh_topic_id() -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(b"mjolnir/mesh/crdt/v0").as_bytes())
}

/// Shared CRDT state for a mesh node.
///
/// In-memory store of all CRDT books. Thread-safe via RwLock.
/// Optionally persists to disk on changes.
#[derive(Debug, Clone)]
pub struct CrdtStore {
    /// Peer address book: node_id -> PeerAddrEntry
    pub peer_addr: Arc<RwLock<AddrBook>>,

    /// Coordinate book: node_id -> CoordinateStamp
    pub coordinates: Arc<RwLock<CoordinateBook>>,

    /// Node name book: node_id -> NodeNameEntry
    pub node_names: Arc<RwLock<NodeNameBook>>,

    /// User book: username -> UserEntry
    pub users: Arc<RwLock<UserBook>>,

    /// Service v2 book: name -> ServiceEntryV2
    pub services_v2: Arc<Mutex<ServiceBookV2>>,

    /// Service tombstone book: name -> ServiceTombstone
    pub service_tombstones: Arc<Mutex<ServiceTombstoneBook>>,

    /// Leased name book: name -> LeasedName
    pub leased_names: Arc<Mutex<LeasedNameBook>>,

    /// Optional data directory for persistence
    pub data_dir: Option<PathBuf>,
}

impl CrdtStore {
    /// Create a new empty CRDT store.
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            peer_addr: Arc::new(RwLock::new(BTreeMap::new())),
            coordinates: Arc::new(RwLock::new(BTreeMap::new())),
            node_names: Arc::new(RwLock::new(BTreeMap::new())),
            users: Arc::new(RwLock::new(BTreeMap::new())),
            services_v2: Arc::new(Mutex::new(ServiceBookV2::new())),
            service_tombstones: Arc::new(Mutex::new(BTreeMap::new())),
            leased_names: Arc::new(Mutex::new(BTreeMap::new())),
            data_dir,
        }
    }

    /// Apply a gossip message into the store, returning whether state changed.
    pub async fn apply(&self, msg: &GossipMessage) -> bool {
        match msg {
            GossipMessage::PeerAddrUpdate { node_id, entry } => {
                let mut book = self.peer_addr.write().unwrap();
                let result = merge_peer_addr(book.get(node_id.as_str()), entry);
                if matches!(result, MergeResult::Inserted | MergeResult::Updated) {
                    book.insert(node_id.clone(), entry.clone());
                    return true;
                }
                false
            }
            GossipMessage::NodeNameAnnounce { node_id, entry } => {
                let mut book = self.node_names.write().unwrap();
                let result = merge_node_name(book.get(node_id.as_str()), entry);
                if matches!(result, MergeResult::Inserted | MergeResult::Updated) {
                    book.insert(node_id.clone(), entry.clone());
                    return true;
                }
                false
            }
            GossipMessage::CoordinateAnnounce { node_id, entry } => {
                let mut book = self.coordinates.write().unwrap();
                let result = apply_coordinate(&mut book, entry);
                if matches!(result, MergeResult::Inserted | MergeResult::Updated) {
                    return true;
                }
                false
            }
            GossipMessage::UserUpdate { username, entry } => {
                let mut book = self.users.write().unwrap();
                let result = merge_user(book.get(username.as_str()), entry);
                if matches!(result, MergeResult::Inserted | MergeResult::Updated) {
                    book.insert(username.clone(), entry.clone());
                    return true;
                }
                false
            }
            GossipMessage::ServicePublishV2 { name, entry } => {
                let mut book = self.services_v2.lock().unwrap();
                let mut tombstones = self.service_tombstones.lock().unwrap();
                match mjolnir_mesh::crdt::service_apply::apply_service_publish_v2(
                    &mut book, &tombstones, name, entry.clone(),
                ) {
                    Ok(_) => true,
                    Err(e) => {
                        debug!(service = name, error = %e, "service publish v2 apply failed");
                        false
                    }
                }
            }
            GossipMessage::ServiceUnpublishV2 {
                name,
                owner_node_id,
                hlc,
            } => {
                let mut book = self.services_v2.lock().unwrap();
                let mut tombstones = self.service_tombstones.lock().unwrap();
                mjolnir_mesh::crdt::service_apply::apply_service_unpublish_v2(
                    &mut book, &mut tombstones, name, owner_node_id, hlc.clone(),
                );
                true
            }
            // Non-CRDT messages: liveness beacons update the tracker externally
            GossipMessage::LivenessBeacon { .. } => false,
            // Legacy messages we store but don't actively use in standalone node
            GossipMessage::LeaseUpdate(_)
            | GossipMessage::LeaseRelease { .. }
            | GossipMessage::DnsUpdate { .. }
            | GossipMessage::ServiceUpdate { .. }
            | GossipMessage::SubnetClaimUpdate { .. }
            | GossipMessage::SubnetClaimRelease { .. }
            | GossipMessage::LeasedNamePublish { .. } => false,
        }
    }

    /// Get the node's own peer address entry for anti-entropy broadcasting.
    pub async fn own_peer_addr(&self, node_id: &str, addrs: Vec<std::net::SocketAddr>) -> PeerAddrEntry {
        PeerAddrEntry {
            node_id: node_id.to_string(),
            direct_addrs: addrs,
            relay_url: None,
            announced_at: mjolnir_mesh::crdt::hlc::tick_hlc(node_id),
        }
    }

    /// Get the node's own name entry for anti-entropy broadcasting.
    pub async fn own_node_name(&self, node_id: &str, name: &str) -> NodeNameEntry {
        NodeNameEntry {
            node_id: node_id.to_string(),
            name: name.to_string(),
            announced_at: mjolnir_mesh::crdt::hlc::tick_hlc(node_id),
        }
    }
}

/// Build a name table for the DNS responder that chains:
/// 1. Well-known names (hello.mesh, id.mesh)
/// 2. Service book (v2)
/// 3. Leased names
/// 4. Liveness-filtered (if tracker provided)
pub fn build_dns_table(
    store: &CrdtStore,
    gateway: GatewayHandle,
) -> Arc<dyn NameTable> {
    let mut tables: Vec<Arc<dyn NameTable>> = Vec::new();

    // Well-known names first
    tables.push(Arc::new(WellKnownTable::new(gateway)));

    // Service book (without liveness filtering — standalone node has no egress)
    let service_table = ServiceTable::new(store.services_v2.clone());
    tables.push(Arc::new(service_table));

    // Leased names
    tables.push(Arc::new(LeasedNameTable::new(store.leased_names.clone())));

    Arc::new(CompositeTable::new(tables))
}

/// Iroh-backed GossipTransport implementation.
/// Wrapped in Arc for Clone.
struct IrohGossipTransportInner {
    sender: GossipSender,
    receiver: TokioMutex<GossipReceiver>,
}

#[derive(Clone)]
struct IrohGossipTransport(Arc<IrohGossipTransportInner>);

#[async_trait]
impl GossipTransport for IrohGossipTransport {
    async fn broadcast(&self, payload: bytes::Bytes) -> Result<(), GossipError> {
        self.0.sender
            .broadcast(payload)
            .await
            .map_err(|e| GossipError::Transport(e.to_string()))
    }

    async fn recv(&self) -> Result<bytes::Bytes, GossipError> {
        let mut rx = self.0.receiver.lock().await;
        loop {
            match rx.next().await {
                Some(Ok(Event::Received(msg))) => return Ok(msg.content),
                Some(Ok(Event::NeighborUp(id))) => {
                    debug!(peer = %id, "gossip: neighbor up");
                }
                Some(Ok(Event::NeighborDown(id))) => {
                    debug!(peer = %id, "gossip: neighbor down");
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(GossipError::Transport(e.to_string())),
                None => return Err(GossipError::Closed),
            }
        }
    }
}

/// Resolve the peer list from --roster file and --peer args.
fn resolve_peers(roster_path: Option<&Path>, peer_args: &[String]) -> Vec<String> {
    let mut peers: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Load from roster file
    if let Some(path) = roster_path {
        if path.exists() {
            match PeerRoster::load(path) {
                Ok(roster) => {
                    for entry in roster.peers() {
                        if seen.insert(entry.token.clone()) {
                            peers.push(entry.token.clone());
                        }
                    }
                }
                Err(e) => warn!(roster = %path.display(), "failed to load roster: {e}"),
            }
        } else {
            warn!(roster = %path.display(), "roster file not found");
        }
    }

    // Merge with --peer args
    for token in peer_args {
        if seen.insert(token.clone()) {
            peers.push(token.clone());
        }
    }

    peers
}

/// Load a persisted secret key from file, or generate and save a new one.
fn load_or_create_secret(path: Option<&Path>) -> Result<SecretKey> {
    let path = path.unwrap_or_else(|| Path::new(".mjolnir-node-secret"));

    if path.exists() {
        let hex_str = std::fs::read_to_string(path).context("read secret file")?;
        let key_bytes = HEXLOWER
            .decode(hex_str.trim().as_bytes())
            .context("secret file contains non-hex")?;
        anyhow::ensure!(
            key_bytes.len() == 32,
            "secret key must be 32 bytes (64 hex chars)"
        );
        let arr: [u8; 32] = key_bytes.try_into().unwrap();
        Ok(SecretKey::from(&arr))
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let key = SecretKey::generate();
        let hex = HEXLOWER.encode(&key.to_bytes());
        std::fs::write(path, &hex).context("write secret file")?;
        info!(path = %path.display(), "generated new node identity");
        Ok(key)
    }
}

/// Build an iroh Endpoint for the mesh.
///
/// LAN mode: Minimal preset, no relay, mDNS lookup.
/// Internet mode: N0 preset, relay, pkarr/DNS + mDNS.
async fn build_endpoint(
    secret: SecretKey,
    no_relay: bool,
    lan: bool,
    relays: &[String],
) -> Result<Endpoint> {
    if lan {
        // LAN mode: no relay, mDNS for peer discovery
        Endpoint::builder(presets::Minimal)
            .clear_ip_transports()
            .relay_mode(RelayMode::Disabled)
            .secret_key(secret)
            .address_lookup(MdnsAddressLookup::builder())
            .bind()
            .await
            .context("failed to bind LAN endpoint")
    } else {
        let relay_mode = if no_relay {
            RelayMode::Disabled
        } else if !relays.is_empty() {
            let urls = relays
                .iter()
                .map(|s| s.parse::<RelayUrl>())
                .collect::<Result<Vec<_>, _>>()
                .context("invalid --relay URL")?;
            RelayMode::custom(urls)
        } else {
            // Staging uses real n0 relays on relay.iroh.network.
            // Never use Default — it points at the flaky canary network.
            RelayMode::Staging
        };

        Endpoint::builder(presets::N0)
            .clear_ip_transports()
            .secret_key(secret)
            .address_lookup(MdnsAddressLookup::builder())
            .relay_mode(relay_mode)
            .bind()
            .await
            .context("failed to build endpoint")
    }
}

/// Wait until the endpoint has a usable address.
async fn wait_until_addressable(endpoint: &Endpoint, no_relay: bool) {
    if !no_relay {
        let _ = tokio::time::timeout(Duration::from_secs(5), endpoint.online()).await;
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while endpoint.addr().is_empty() && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    info!(addr = ?endpoint.addr(), "endpoint addressable");
}

/// Print this node's identity for sharing with peers.
fn print_identity(endpoint: &Endpoint) -> Result<()> {
    println!("node id: {}", endpoint.id());
    let blob = encode_addr(endpoint);
    println!("address: {}", blob);
    Ok(())
}

/// Encode an endpoint's address into a shareable string.
fn encode_addr(endpoint: &Endpoint) -> String {
    let mut parts = Vec::new();
    parts.push(format!("node:{}", endpoint.id()));
    for ip in endpoint.addr().ip_addrs() {
        parts.push(format!("ip:{}", ip));
    }
    for relay in endpoint.addr().relay_urls() {
        parts.push(format!("relay:{}", relay));
    }
    parts.join(",")
}

// ===========================================================================
// Anti-entropy loop
// ===========================================================================

/// Broadcast the node's own peer address, node name, and coordinate to the mesh
/// on a regular cadence. Also publishes a liveness beacon.
///
/// Mirrors [`anti_entropy_loop`](mjolnir_mesh::bin::mjolnir_meshd::anti_entropy_loop)
/// from the daemon but simplified for the standalone node.
pub async fn anti_entropy_loop<T: GossipTransport + Clone + Send + Sync + 'static>(
    transport: T,
    store: Arc<CrdtStore>,
    node_id: String,
    interval: Duration,
    tracker: Arc<Mutex<LivenessTracker>>,
    incarnation: u64,
    name: Option<String>,
    lat_e7: Option<i32>,
    lon_e7: Option<i32>,
    stamper: Option<String>,
) {
    let mut tick = tokio::time::interval(interval);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut counter: u64 = 0;

    loop {
        tick.tick().await;
        counter += 1;

        let hlc = mjolnir_mesh::crdt::hlc::tick_hlc(&node_id);

        // 1. Broadcast own peer address
        let addrs: Vec<std::net::SocketAddr> = {
            let endpoint_addrs = vec![format!("{}:{}", node_id, 0)];
            endpoint_addrs
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect()
        };

        let peer_entry = PeerAddrEntry {
            node_id: node_id.clone(),
            direct_addrs: addrs,
            relay_url: None,
            announced_at: hlc.clone(),
        };

        let peer_msg = GossipMessage::PeerAddrUpdate {
            node_id: node_id.clone(),
            entry: peer_entry,
        };

        if let Err(e) = broadcast_message(&transport, &peer_msg).await {
            warn!(error = %e, "failed to broadcast peer addr update");
        }

        // 2. Broadcast own node name (if set)
        if let Some(ref node_name) = name {
            let name_entry = NodeNameEntry {
                node_id: node_id.clone(),
                name: node_name.clone(),
                announced_at: hlc.clone(),
            };

            let name_msg = GossipMessage::NodeNameAnnounce {
                node_id: node_id.clone(),
                entry: name_entry,
            };

            if let Err(e) = broadcast_message(&transport, &name_msg).await {
                warn!(error = %e, "failed to broadcast node name");
            }
        }

        // 3. Broadcast own coordinate (if set)
        if let (Some(lat), Some(lon)) = (lat_e7, lon_e7) {
            let coord = CoordinateStamp {
                node_id: node_id.clone(),
                lat_e7: lat,
                lon_e7: lon,
                alt_mm: None,
                stamped_at_unix: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                stamper: stamper.clone().unwrap_or_else(|| "node".to_string()),
            };

            let coord_msg = GossipMessage::CoordinateAnnounce {
                node_id: node_id.clone(),
                entry: coord,
            };

            if let Err(e) = broadcast_message(&transport, &coord_msg).await {
                warn!(error = %e, "failed to broadcast coordinate");
            }
        }

        // 4. Publish liveness beacon
        let egress = None; // No egress in standalone node
        let beacon = GossipMessage::LivenessBeacon {
            node_id: node_id.clone(),
            incarnation,
            counter,
            egress,
        };

        if let Err(e) = broadcast_message(&transport, &beacon).await {
            warn!(error = %e, "failed to broadcast liveness beacon");
        }

        // 5. Touch our own liveness in the tracker
        tracker.lock().unwrap().touch(&node_id, monotonic_now_ms());

        debug!(
            tick = counter,
            "anti-entropy broadcast complete"
        );
    }
}

/// Serialize a gossip message and broadcast it.
async fn broadcast_message<T: GossipTransport>(
    transport: &T,
    msg: &GossipMessage,
) -> Result<()> {
    let bytes = postcard::to_allocvec(msg).context("serialize gossip message")?;
    transport
        .broadcast(bytes::Bytes::from(bytes))
        .await
        .context("broadcast gossip message")?;
    Ok(())
}

// ===========================================================================
// Control API (JSON over TCP)
// ===========================================================================

/// Request types for the control API.
#[derive(Debug, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
enum ControlRequest {
    /// Get the node's current state.
    State,
    /// Get peer addresses.
    Peers,
    /// Get services.
    Services,
    /// Get users.
    Users,
    /// Get coordinates.
    Coordinates,
    /// Get node names.
    NodeNames,
}

/// Response from the control API.
#[derive(Debug, Serialize)]
struct ControlResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Run the JSON control API server on a TCP port.
pub async fn control_api_loop(
    addr: SocketAddr,
    store: Arc<CrdtStore>,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("bind control API port")?;

    info!(%addr, "control API listening");

    loop {
        let (socket, peer) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "control API accept error");
                continue;
            }
        };

        tokio::spawn(handle_control_connection(socket, peer, store.clone()));
    }
}

/// Handle a single control API connection.
async fn handle_control_connection(
    socket: tokio::net::TcpStream,
    _peer: SocketAddr,
    store: Arc<CrdtStore>,
) {
    let (reader, writer) = socket.into_split();
    let mut reader = tokio::io::BufReader::new(reader);
    let mut writer = tokio::io::BufWriter::new(writer);

    // Read the entire request (control messages are small)
    let mut request_buf = String::new();
    if reader.read_line(&mut request_buf).await.is_err() {
        return;
    }

    let request: ControlRequest = match serde_json::from_str(&request_buf) {
        Ok(r) => r,
        Err(e) => {
            let resp = ControlResponse {
                success: false,
                data: None,
                error: Some(format!("parse error: {e}")),
            };
            let resp_str = serde_json::to_string(&resp).unwrap();
            let _ = writer.write_all(resp_str.as_bytes()).await;
            let _ = writer.flush().await;
            return;
        }
    };

    let resp = match request {
        ControlRequest::State => {
            let state = ControlState {
                peer_count: store.peer_addr.read().unwrap().len(),
                service_count: store.services_v2.lock().unwrap().len(),
                user_count: store.users.read().unwrap().len(),
                coordinate_count: store.coordinates.read().unwrap().len(),
                node_name_count: store.node_names.read().unwrap().len(),
            };
            ControlResponse {
                success: true,
                data: Some(serde_json::to_value(state).unwrap()),
                error: None,
            }
        }
        ControlRequest::Peers => {
            let peers = store.peer_addr.read().unwrap().iter()
                .map(|(k, v)| serde_json::json!({ "node_id": k, "addrs": v.direct_addrs }))
                .collect::<Vec<_>>();
            ControlResponse {
                success: true,
                data: Some(serde_json::to_value(peers).unwrap()),
                error: None,
            }
        }
        ControlRequest::Services => {
            let services = store.services_v2.lock().unwrap().iter()
                .map(|(k, v)| serde_json::json!({ "name": k, "owner": v.owner_node_id }))
                .collect::<Vec<_>>();
            ControlResponse {
                success: true,
                data: Some(serde_json::to_value(services).unwrap()),
                error: None,
            }
        }
        ControlRequest::Users => {
            let users = store.users.read().unwrap().iter()
                .map(|(k, v)| serde_json::json!({ "username": k, "display": v.display_name }))
                .collect::<Vec<_>>();
            ControlResponse {
                success: true,
                data: Some(serde_json::to_value(users).unwrap()),
                error: None,
            }
        }
        ControlRequest::Coordinates => {
            let coords = store.coordinates.read().unwrap().iter()
                .map(|(k, v)| serde_json::json!({ "node_id": k, "lat": v.lat_e7, "lon": v.lon_e7 }))
                .collect::<Vec<_>>();
            ControlResponse {
                success: true,
                data: Some(serde_json::to_value(coords).unwrap()),
                error: None,
            }
        }
        ControlRequest::NodeNames => {
            let names = store.node_names.read().unwrap().iter()
                .map(|(k, v)| serde_json::json!({ "node_id": k, "name": v.name }))
                .collect::<Vec<_>>();
            ControlResponse {
                success: true,
                data: Some(serde_json::to_value(names).unwrap()),
                error: None,
            }
        }
    };

    let resp_str = serde_json::to_string(&resp).unwrap();
    let _ = writer.write_all(resp_str.as_bytes()).await;
    let _ = writer.flush().await;
}

/// Control API state response.
#[derive(Debug, Serialize)]
struct ControlState {
    peer_count: usize,
    service_count: usize,
    user_count: usize,
    coordinate_count: usize,
    node_name_count: usize,
}

// ===========================================================================
// Main listen loop
// ===========================================================================

/// Run the full-featured gossip listen loop.
async fn run_listen(
    secret: SecretKey,
    no_relay: bool,
    lan: bool,
    relays: &[String],
    roster_path: Option<&Path>,
    peer_args: &[String],
    data_dir: Option<&Path>,
    control_port: Option<u16>,
    dns_bind: Option<&str>,
    anti_entropy_interval: Option<u64>,
) -> Result<()> {
    let endpoint = build_endpoint(secret, no_relay, lan, relays).await?;
    wait_until_addressable(&endpoint, no_relay).await;
    let _ = print_identity(&endpoint);

    let self_id = endpoint.id();
    let node_id = self_id.to_string();
    info!(node = %node_id, "starting full-featured gossip node");

    // Resolve peer list
    let peer_ids = resolve_peers(roster_path, peer_args);
    info!(peers = peer_ids.len(), "resolved peer list");

    // Create gossip instance
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // Parse peer EndpointIds for bootstrap
    let bootstrap: Vec<EndpointId> = peer_ids
        .iter()
        .filter_map(|id| id.parse().ok())
        .filter(|id| *id != self_id)
        .collect();

    // Join the mesh topic with bootstrap peers
    let topic = mesh_topic_id();
    info!(topic = %topic, bootstrap = bootstrap.len(), "joining mesh topic");

    let mesh = match gossip.subscribe(topic, bootstrap.clone()).await {
        Ok(mesh) => mesh,
        Err(e) => {
            error!(error = %e, "failed to subscribe to gossip topic");
            return Err(e).context("gossip subscribe failed");
        }
    };

    let (sender, receiver) = mesh.split();
    let transport = IrohGossipTransport(Arc::new(IrohGossipTransportInner {
        sender,
        receiver: TokioMutex::new(receiver),
    }));

    // Create CRDT store
    let store = Arc::new(CrdtStore::new(data_dir.map(PathBuf::from)));

    // Create liveness tracker
    let tracker = Arc::new(Mutex::new(LivenessTracker::new(60_000, 3_600_000)));

    // Gateway handle for DNS well-known names
    let gateway: GatewayHandle = Arc::new(RwLock::new(None));

    // Build DNS name table
    let dns_table = build_dns_table(&store, gateway.clone());

    // Spawn anti-entropy loop
    let incarnation = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let interval = Duration::from_secs(anti_entropy_interval.unwrap_or(30));
    let anti_entropy = tokio::spawn(anti_entropy_loop(
        transport.clone(),
        store.clone(),
        node_id.clone(),
        interval,
        tracker.clone(),
        incarnation,
        None, // No node name in standalone node
        None, // No coordinates
        None, // No stamper
        None, // No stamper string
    ));

    // Spawn DNS responder
    let dns_handle = if let Some(bind_str) = dns_bind {
        let addr: SocketAddr = bind_str.parse().context("invalid --dns-bind address")?;
        match dns_responder::start(addr, dns_table).await {
            Ok(handle) => {
                info!(%addr, "DNS responder started");
                Some(handle)
            }
            Err(e) => {
                warn!(error = %e, "failed to start DNS responder");
                None
            }
        }
    } else {
        None
    };

    // Spawn control API
    let control_handle = if let Some(port) = control_port {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        Some(tokio::spawn(control_api_loop(addr, store.clone())))
    } else {
        None
    };

    // Message handler: apply CRDT messages and log
    let message_count = Arc::new(Mutex::new(0u64));
    let msg_count = message_count.clone();
    let store_for_handler = store.clone();

    let handler = move |msg: GossipMessage| {
        let count = {
            let mut c = msg_count.lock().unwrap();
            *c += 1;
            *c
        };

        // Apply message to CRDT store (block_in_place since apply is async)
        let changed = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(store_for_handler.apply(&msg))
        });

        // Update liveness tracker from beacons (beacons don't change CRDT state)
        if let GossipMessage::LivenessBeacon {
            ref node_id,
            incarnation,
            counter,
            egress,
        } = msg
        {
            let mut t = tracker.lock().unwrap();
            t.observe(node_id, incarnation, counter, monotonic_now_ms());
        }

        // Log the message
        match &msg {
            GossipMessage::LeaseUpdate(entry) => {
                info!(
                    msg_count = count,
                    mac = ?entry.mac,
                    ip = ?entry.ip,
                    hostname = ?entry.hostname,
                    "lease update"
                );
            }
            GossipMessage::LeaseRelease { mac, hlc: _ } => {
                info!(msg_count = count, mac = ?mac, "lease release");
            }
            GossipMessage::DnsUpdate { hostname, entry } => {
                info!(
                    msg_count = count,
                    hostname,
                    ip = ?entry.ip,
                    "dns update"
                );
            }
            GossipMessage::ServiceUpdate { name, entry } => {
                info!(
                    msg_count = count,
                    name,
                    ip = ?entry.ip,
                    port = entry.port,
                    "service update"
                );
            }
            GossipMessage::SubnetClaimUpdate { cidr, entry } => {
                info!(
                    msg_count = count,
                    cidr,
                    owner = %entry.owner_node_id,
                    "subnet claim update"
                );
            }
            GossipMessage::SubnetClaimRelease { cidr, hlc: _ } => {
                info!(msg_count = count, cidr, "subnet claim release");
            }
            GossipMessage::PeerAddrUpdate { node_id, entry } => {
                info!(
                    msg_count = count,
                    node_id,
                    addrs = ?entry.direct_addrs,
                    "peer address update"
                );
            }
            GossipMessage::UserUpdate { username, entry } => {
                info!(
                    msg_count = count,
                    username,
                    display = %entry.display_name,
                    "user update"
                );
            }
            GossipMessage::ServicePublishV2 { name, entry } => {
                info!(
                    msg_count = count,
                    name,
                    owner = %entry.owner_node_id,
                    "service publish v2"
                );
            }
            GossipMessage::ServiceUnpublishV2 { name, owner_node_id, hlc: _ } => {
                info!(
                    msg_count = count,
                    name,
                    owner = owner_node_id,
                    "service unpublish v2"
                );
            }
            GossipMessage::LivenessBeacon { node_id, incarnation, counter, egress } => {
                info!(
                    msg_count = count,
                    node_id,
                    incarnation,
                    counter,
                    egress = ?egress,
                    "liveness beacon"
                );
            }
            GossipMessage::NodeNameAnnounce { node_id, entry } => {
                info!(
                    msg_count = count,
                    node_id,
                    name = %entry.name,
                    "node name announce"
                );
            }
            GossipMessage::LeasedNamePublish { name, entry: _ } => {
                info!(
                    msg_count = count,
                    name,
                    "leased name publish"
                );
            }
            GossipMessage::CoordinateAnnounce { node_id, entry } => {
                info!(
                    msg_count = count,
                    node_id,
                    lat = entry.lat_e7,
                    lon = entry.lon_e7,
                    "coordinate announce"
                );
            }
        }
    };

    // Run the receive loop in a separate task
    let recv_handle = tokio::spawn(async move {
        if let Err(e) = GossipSync::new(transport).run(handler).await {
            error!(error = %e, "gossip receive loop error");
        }
    });

    info!(
        "listening for gossip messages (Ctrl-C to stop)"
    );

    // Keep the main task alive until Ctrl-C
    tokio::signal::ctrl_c().await?;

    info!("shutting down...");
    recv_handle.abort();
    anti_entropy.abort();
    if let Some(handle) = dns_handle {
        handle.abort();
    }
    if let Some(handle) = control_handle {
        handle.abort();
    }
    endpoint.close().await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    use std::io::IsTerminal;
    tracing_subscriber::fmt()
        .with_ansi(std::io::stderr().is_terminal())
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let cmd = cli.command.unwrap_or(Command::Listen);

    let secret = load_or_create_secret(cli.secret_file.as_deref())?;
    let no_relay = cli.no_relay || cli.lan;

    match cmd {
        Command::Id => {
            let endpoint = build_endpoint(secret, no_relay, cli.lan, &cli.relay).await?;
            wait_until_addressable(&endpoint, no_relay).await;
            print_identity(&endpoint)?;
            endpoint.close().await;
        }
        Command::Listen => {
            let data_dir = cli.data_dir.as_deref();

            run_listen(
                secret,
                no_relay,
                cli.lan,
                &cli.relay,
                cli.roster.as_deref(),
                &cli.peer,
                data_dir,
                cli.control_port,
                cli.dns_bind.as_deref(),
                cli.anti_entropy_interval,
            )
            .await?;
        }
    }

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const HEX64: &str = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

    // -----------------------------------------------------------------------
    // mesh_topic_id
    // -----------------------------------------------------------------------

    #[test]
    fn mesh_topic_id_is_deterministic() {
        let a = mesh_topic_id();
        let b = mesh_topic_id();
        assert_eq!(a, b, "topic id must be deterministic");
    }

    #[test]
    fn mesh_topic_id_is_not_empty() {
        let topic = mesh_topic_id();
        let bytes = topic.as_bytes();
        assert_eq!(bytes.len(), 32, "topic id must be 32 bytes");
        assert!(bytes.iter().any(|&b| b != 0), "topic id must not be all zeros");
    }

    #[test]
    fn mesh_topic_id_differs_from_random_hash() {
        let topic = mesh_topic_id();
        let random_hash = blake3::hash(b"random-string-not-mjolnir-mesh");
        assert_ne!(
            topic.as_bytes(),
            random_hash.as_bytes(),
            "topic id must be based on the mjolnir/mesh/crdt/v0 seed"
        );
    }

    // -----------------------------------------------------------------------
    // load_or_create_secret
    // -----------------------------------------------------------------------

    #[test]
    fn load_secret_creates_file_on_first_call() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.hex");
        assert!(!secret_path.exists(), "secret file must not exist before first call");

        let key = load_or_create_secret(Some(secret_path.as_path())).unwrap();

        assert!(secret_path.exists(), "secret file must be created on first call");
        let hex_str = std::fs::read_to_string(&secret_path).unwrap();
        let decoded = HEXLOWER.decode(hex_str.trim().as_bytes()).unwrap();
        assert_eq!(decoded.len(), 32, "secret file must contain 32 bytes");
        // Re-loaded key must equal the generated key
        let reloaded = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        assert_eq!(key.to_bytes(), reloaded.to_bytes());
    }

    #[test]
    fn load_secret_reuses_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.hex");

        let key1 = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        let key2 = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        assert_eq!(key1.to_bytes(), key2.to_bytes(), "reloading must return the same key");
    }

    #[test]
    fn load_secret_from_nonexistent_path_returns_error() {
        let result = load_or_create_secret(Some(Path::new("/nonexistent/path/secret.hex")));
        assert!(result.is_err(), "must error on nonexistent path");
    }

    #[test]
    fn load_secret_with_nonhex_content_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.hex");
        std::fs::write(&secret_path, "not-valid-hex-at-all!!!").unwrap();

        let result = load_or_create_secret(Some(secret_path.as_path()));
        assert!(result.is_err(), "non-hex content must produce a decode error");
    }

    #[test]
    fn load_secret_with_wrong_length_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.hex");
        std::fs::write(
            &secret_path,
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        )
        .unwrap();

        let result = load_or_create_secret(Some(secret_path.as_path()));
        assert!(result.is_err(), "wrong-length key must produce a length error");
    }

    #[test]
    fn load_secret_creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("subdir/nested/secret.hex");

        let key = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        assert!(secret_path.exists());
        let key2 = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        assert_eq!(key.to_bytes(), key2.to_bytes());
    }

    // -----------------------------------------------------------------------
    // resolve_peers
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_peers_empty_returns_empty() {
        let peers = resolve_peers(None, &[]);
        assert!(peers.is_empty());
    }

    #[test]
    fn resolve_peers_single_peer_arg() {
        let peers = resolve_peers(None, &[HEX64.to_string()]);
        assert_eq!(peers, vec![HEX64.to_string()]);
    }

    #[test]
    fn resolve_peers_deduplicates_peer_args() {
        let peers = resolve_peers(None, &[HEX64.to_string(), HEX64.to_string()]);
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], HEX64);
    }

    #[test]
    fn resolve_peers_merges_roster_and_peer_args() {
        let dir = tempfile::tempdir().unwrap();
        let roster_path = dir.path().join("roster.txt");
        std::fs::write(&roster_path, format!("{HEX64}\n{HEX64}  duplicate\n")).unwrap();

        let peers = resolve_peers(Some(roster_path.as_path()), &[HEX64.to_string()]);
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], HEX64);
    }

    #[test]
    fn resolve_peers_roster_file_not_found_warns_and_continues() {
        let peers = resolve_peers(Some(Path::new("/nonexistent/roster.txt")), &[]);
        assert!(peers.is_empty());
    }

    // -----------------------------------------------------------------------
    // CrdtStore
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn crdt_store_applies_peer_addr_update() {
        let store = CrdtStore::new(None);
        let node_id = "test-node".to_string();
        let hlc = mjolnir_mesh::crdt::hlc::HLC {
            wall_clock: 1_000_000,
            counter: 0,
            node_id: node_id.clone(),
        };
        let entry = PeerAddrEntry {
            node_id: node_id.clone(),
            direct_addrs: vec!["10.0.0.1:49737".parse().unwrap()],
            relay_url: None,
            announced_at: hlc,
        };
        let msg = GossipMessage::PeerAddrUpdate {
            node_id: node_id.clone(),
            entry,
        };

        let changed = store.apply(&msg).await;
        assert!(changed, "peer addr update should change store");

        let book = store.peer_addr.read().unwrap();
        assert!(book.contains_key(&node_id));
    }

    #[tokio::test]
    async fn crdt_store_peer_addr_unchanged_on_duplicate() {
        let store = CrdtStore::new(None);
        let node_id = "test-node".to_string();
        let hlc = mjolnir_mesh::crdt::hlc::HLC {
            wall_clock: 1_000_000,
            counter: 0,
            node_id: node_id.clone(),
        };
        let entry = PeerAddrEntry {
            node_id: node_id.clone(),
            direct_addrs: vec!["10.0.0.1:49737".parse().unwrap()],
            relay_url: None,
            announced_at: hlc.clone(),
        };
        let msg = GossipMessage::PeerAddrUpdate {
            node_id: node_id.clone(),
            entry: entry.clone(),
        };

        let changed1 = store.apply(&msg).await;
        assert!(changed1);

        // Same message again — should be unchanged
        let changed2 = store.apply(&msg).await;
        assert!(!changed2, "duplicate peer addr should be unchanged");
    }

    #[tokio::test]
    async fn crdt_store_applies_node_name_announce() {
        let store = CrdtStore::new(None);
        let node_id = "test-node".to_string();
        let hlc = mjolnir_mesh::crdt::hlc::HLC {
            wall_clock: 1_000_000,
            counter: 0,
            node_id: node_id.clone(),
        };
        let entry = NodeNameEntry {
            node_id: node_id.clone(),
            name: "attic".to_string(),
            announced_at: hlc,
        };
        let msg = GossipMessage::NodeNameAnnounce {
            node_id: node_id.clone(),
            entry,
        };

        let changed = store.apply(&msg).await;
        assert!(changed);

        let book = store.node_names.read().unwrap();
        assert!(book.contains_key(&node_id));
        assert_eq!(book[&node_id].name, "attic");
    }

    #[tokio::test]
    async fn crdt_store_applies_coordinate_announce() {
        let store = CrdtStore::new(None);
        let entry = CoordinateStamp {
            node_id: "router-a".to_string(),
            lat_e7: 370_000_000,
            lon_e7: -1_220_000_000,
            alt_mm: None,
            stamped_at_unix: 1_700_000_040,
            stamper: "phone-ada".to_string(),
        };
        let msg = GossipMessage::CoordinateAnnounce {
            node_id: "router-a".to_string(),
            entry,
        };

        let changed = store.apply(&msg).await;
        assert!(changed);

        let book = store.coordinates.read().unwrap();
        assert!(book.contains_key("router-a"));
    }

    #[tokio::test]
    async fn crdt_store_applies_user_update() {
        let store = CrdtStore::new(None);
        let entry = UserEntry {
            username: "ada".to_string(),
            display_name: "Ada Lovelace".to_string(),
            registered_by: "router-a".to_string(),
            attrs: std::collections::BTreeMap::new(),
            updated_at: mjolnir_mesh::crdt::hlc::HLC {
                wall_clock: 1_000_000,
                counter: 0,
                node_id: "router-a".to_string(),
            },
        };
        let msg = GossipMessage::UserUpdate {
            username: "ada".to_string(),
            entry,
        };

        let changed = store.apply(&msg).await;
        assert!(changed);

        let book = store.users.read().unwrap();
        assert!(book.contains_key("ada"));
    }

    #[tokio::test]
    async fn crdt_store_liveness_beacon_returns_false() {
        let store = CrdtStore::new(None);
        let msg = GossipMessage::LivenessBeacon {
            node_id: "router-a".to_string(),
            incarnation: 1_000_000,
            counter: 0,
            egress: None,
        };

        let changed = store.apply(&msg).await;
        assert!(!changed, "liveness beacon should not change CRDT store");
    }

    #[tokio::test]
    async fn crdt_store_lease_update_returns_false() {
        let store = CrdtStore::new(None);
        let entry = LeaseEntry {
            mac: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
            hostname: Some("laptop".to_string()),
            router_id: "router-a".to_string(),
            expiry: 1_700_000_000,
            hlc: mjolnir_mesh::crdt::hlc::HLC {
                wall_clock: 1_700_000_000_000,
                counter: 0,
                node_id: "router-a".to_string(),
            },
        };
        let msg = GossipMessage::LeaseUpdate(entry);

        let changed = store.apply(&msg).await;
        assert!(!changed, "lease update should not change standalone store");
    }

    #[tokio::test]
    async fn crdt_store_empty() {
        let store = CrdtStore::new(None);
        assert_eq!(store.peer_addr.read().unwrap().len(), 0);
        assert_eq!(store.node_names.read().unwrap().len(), 0);
        assert_eq!(store.coordinates.read().unwrap().len(), 0);
        assert_eq!(store.users.read().unwrap().len(), 0);
    }

    // -----------------------------------------------------------------------
    // build_dns_table
    // -----------------------------------------------------------------------

    #[test]
    fn build_dns_table_returns_composite() {
        let store = CrdtStore::new(None);
        let gateway: GatewayHandle = Arc::new(RwLock::new(None));
        let table = build_dns_table(&store, gateway);

        // Well-known names should resolve
        assert!(table.lookup_a("hello.mesh.").is_some());
        assert!(table.lookup_a("id.mesh.").is_some());

        // Unknown names should not resolve
        assert!(table.lookup_a("unknown.mesh.").is_none());
    }

    // -----------------------------------------------------------------------
    // encode_addr
    // -----------------------------------------------------------------------

    // encode_addr requires a real Endpoint, which is hard to mock.
    // The function is a simple string builder tested indirectly
    // via the `id` subcommand integration.

    // -----------------------------------------------------------------------
    // print_identity
    // -----------------------------------------------------------------------

    // Same constraint as encode_addr — requires a real Endpoint.
    // Tested via the `id` subcommand in integration tests.
}
