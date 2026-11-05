// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! `mjolnir-node` — standalone iroh mesh node for the mjolnir-mesh private network.
//!
//! This binary joins the mesh gossip topic and prints received CRDT messages
//! to stdout. It does NOT run babeld, TUN tunnels, or DNS — it is a gossip-only
//! observer that can run on any platform (macOS, Linux, containers).
//!
//! Subcommands:
//!   listen    join the gossip swarm, print received messages (default)
//!   id        print this node's EndpointId and address blob

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
use mjolnir_mesh::crdt::sync::{GossipError, GossipSync, GossipTransport};
use mjolnir_mesh::roster::PeerRoster;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Standalone iroh mesh node — gossip-only observer.
#[derive(Parser)]
#[command(name = "mjolnir-node", about = "Join the mjolnir-mesh gossip swarm")]
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print this node's EndpointId and a shareable address blob.
    Id,
    /// Join the gossip swarm and print received messages (default).
    Listen,
}

/// The fixed gossip topic for a mesh's CRDT overlay.
/// Same constant hash as mjolnir-meshd.
fn mesh_topic_id() -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(b"mjolnir/mesh/crdt/v0").as_bytes())
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
            run_listen(
                secret,
                no_relay,
                cli.lan,
                &cli.relay,
                cli.roster.as_deref(),
                &cli.peer,
            )
            .await?;
        }
    }

    Ok(())
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

/// Iroh-backed GossipTransport implementation.
struct IrohGossipTransport {
    sender: GossipSender,
    receiver: Mutex<GossipReceiver>,
}

#[async_trait]
impl GossipTransport for IrohGossipTransport {
    async fn broadcast(&self, payload: bytes::Bytes) -> Result<(), GossipError> {
        self.sender
            .broadcast(payload)
            .await
            .map_err(|e| GossipError::Transport(e.to_string()))
    }

    async fn recv(&self) -> Result<bytes::Bytes, GossipError> {
        let mut rx = self.receiver.lock().await;
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

/// Run the gossip listen loop: join the topic and print received messages.
async fn run_listen(
    secret: SecretKey,
    no_relay: bool,
    lan: bool,
    relays: &[String],
    roster_path: Option<&Path>,
    peer_args: &[String],
) -> Result<()> {
    let endpoint = build_endpoint(secret, no_relay, lan, relays).await?;
    wait_until_addressable(&endpoint, no_relay).await;
    let _ = print_identity(&endpoint);

    let self_id = endpoint.id();
    info!(node = %self_id, "starting gossip listen loop");

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
    let transport = IrohGossipTransport {
        sender,
        receiver: Mutex::new(receiver),
    };
    let sync = GossipSync::new(transport);

    // Message handler: decode and print each received gossip message
    let message_count = Arc::new(Mutex::new(0u64));
    let msg_count = message_count.clone();

    info!("listening for gossip messages (Ctrl-C to stop)");

    // Run the gossip receive loop
    let handler = move |msg: GossipMessage| {
        let count = tokio::task::block_in_place(|| {
            let mut c = tokio::runtime::Handle::current().block_on(msg_count.lock());
            *c += 1;
            *c
        });
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
        if let Err(e) = sync.run(handler).await {
            error!(error = %e, "gossip receive loop error");
        }
    });

    // Keep the main task alive until Ctrl-C
    tokio::signal::ctrl_c().await?;

    info!("shutting down...");
    recv_handle.abort();
    endpoint.close().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX64: &str = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

    // ---------------------------------------------------------------------------
    // mesh_topic_id — deterministic topic hash
    // ---------------------------------------------------------------------------

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
        assert!(
            bytes.iter().any(|&b| b != 0),
            "topic id must not be all zeros"
        );
    }

    // ---------------------------------------------------------------------------
    // load_or_create_secret — identity persistence
    // ---------------------------------------------------------------------------

    #[test]
    fn load_secret_creates_file_on_first_call() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.hex");
        assert!(
            !secret_path.exists(),
            "secret file must not exist before first call"
        );

        let key = load_or_create_secret(Some(secret_path.as_path())).unwrap();

        assert!(
            secret_path.exists(),
            "secret file must be created on first call"
        );
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

        // Generate and save the first key
        let key1 = load_or_create_secret(Some(secret_path.as_path())).unwrap();

        // Load again — must return the same key
        let key2 = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        assert_eq!(
            key1.to_bytes(),
            key2.to_bytes(),
            "reloading must return the same key"
        );
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
        assert!(
            result.is_err(),
            "non-hex content must produce a decode error"
        );
    }

    #[test]
    fn load_secret_with_wrong_length_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.hex");
        // 20 bytes hex (40 chars) — too short
        std::fs::write(&secret_path, "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2").unwrap();

        let result = load_or_create_secret(Some(secret_path.as_path()));
        assert!(
            result.is_err(),
            "wrong-length key must produce a length error"
        );
    }

    #[test]
    fn load_secret_creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("subdir/nested/secret.hex");

        let key = load_or_create_secret(Some(secret_path.as_path())).unwrap();

        assert!(secret_path.exists());
        // Verify we can reload from the same path
        let key2 = load_or_create_secret(Some(secret_path.as_path())).unwrap();
        assert_eq!(key.to_bytes(), key2.to_bytes());
    }

    // ---------------------------------------------------------------------------
    // resolve_peers — peer list resolution
    // ---------------------------------------------------------------------------

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
        std::fs::write(
            &roster_path,
            format!("{HEX64}\n{HEX64}  duplicate\n"),
        )
        .unwrap();

        let peers = resolve_peers(Some(roster_path.as_path()), &[HEX64.to_string()]);
        // Roster provides HEX64 (first-wins dedup), peer arg is deduped against roster
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], HEX64);
    }

    #[test]
    fn resolve_peers_roster_file_not_found_warns_and_continues() {
        let peers = resolve_peers(Some(Path::new("/nonexistent/roster.txt")), &[]);
        assert!(peers.is_empty());
    }

    // ---------------------------------------------------------------------------
    // print_identity — address encoding
    // ---------------------------------------------------------------------------

    // We cannot easily test print_identity without a real Endpoint, but we can
    // test encode_addr's logic indirectly via the mock endpoint patterns
    // already tested in the mesh module. The encode_addr function is a simple
    // string builder — the real test surface is the CLI integration.

    // ---------------------------------------------------------------------------
    // mesh_topic_id — uniqueness from seed
    // ---------------------------------------------------------------------------

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
}
