//! `mjolnir-meshd` — headless iroh-transport router daemon (P0: connectivity MVP).
//!
//! Phase 0 proves the core value prop on real hardware: a persistent iroh
//! identity plus QUIC connectivity (with NAT traversal via relays) between two
//! nodes. There is deliberately **no TUN** yet — that is P1 — so this binary
//! can validate iroh-in-a-RouterOS-container *before* the unverified
//! TUN-in-container question. See beads mjolnir-mesh-tr6 / mjolnir-mesh-02g.
//!
//! Subcommands:
//!   id                 print this node's EndpointId and a shareable address blob
//!   listen             accept inbound connections, echo ping datagrams
//!   connect <addr>     dial a peer by address blob, measure a datagram round-trip

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bytes::Bytes;
use clap::{Parser, Subcommand};
use iroh::endpoint::presets;
use iroh::endpoint::Connection;
use iroh_mdns_address_lookup::MdnsAddressLookup;
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, RelayUrl, SecretKey};
use mjolnir_mesh::tun::{spawn_tunnel, DatagramConn, EncapError};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// ALPN for the P0 mesh connectivity probe. Bumped per protocol revision.
const MESH_ALPN: &[u8] = b"mjolnir/mesh/v0";

/// ALPN for the P1 L3 tunnel (TUN packets over iroh datagrams).
const TUN_ALPN: &[u8] = b"mjolnir/mesh/tun/v0";

/// UDP port the tunnel reachability probe echoes on (bound to the TUN /31 addr).
const TUN_PROBE_PORT: u16 = 9999;

/// Datagram payload used to prove an end-to-end round-trip.
const PING: &[u8] = b"mjolnir-ping";

#[derive(Parser)]
#[command(
    name = "mjolnir-meshd",
    about = "Headless iroh-transport mesh daemon (P0 connectivity)"
)]
struct Cli {
    /// Path to the persisted node secret key (hex). Generated on first run if
    /// absent. If omitted, falls back to the IROH_SECRET env var, then to an
    /// ephemeral key (logged as a warning — identity won't survive restart).
    #[arg(long, global = true)]
    secret_file: Option<PathBuf>,

    /// Disable iroh relays (direct/LAN only). Useful for offline/LAN meshes and
    /// for same-host testing without depending on public relay servers.
    #[arg(long, global = true)]
    no_relay: bool,

    /// Bind to a specific socket address (e.g. 127.0.0.1:0 for a loopback-only
    /// test). Default is iroh's wildcard bind.
    #[arg(long, global = true)]
    bind: Option<SocketAddr>,

    /// LAN-direct mode: discover peers via mDNS on the local network, no relay,
    /// no pkarr/DNS, no internet. Connect by bare node id; addresses are found
    /// over the LAN. Implies --no-relay. For same-switch swarms.
    #[arg(long, global = true)]
    lan: bool,

    /// Relay server URL(s) to use (repeatable), e.g. a self-hosted relay. If
    /// omitted, uses n0's staging relays. NOTE: iroh 0.96's "Default" points at
    /// the flaky canary network, so we never use it.
    #[arg(long, global = true)]
    relay: Vec<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print this node's EndpointId and a shareable address blob.
    Id,
    /// Listen for inbound mesh connections and echo ping datagrams. Runs until Ctrl-C.
    Listen,
    /// Dial a peer (address blob from `id`/`listen`) and measure a round-trip.
    Connect {
        /// Address blob printed by the peer's `id` or `listen`.
        addr: String,
    },
    /// Probe whether a TUN device can be created in this environment (e.g.
    /// inside a RouterOS container). Creates a throwaway /31 link and tears it
    /// down. This is the gating check for the L3 data plane (P1).
    TunTest,
    /// P1: listen for a peer and bring up a per-peer /31 TUN tunnel over iroh.
    /// Runs until Ctrl-C; echoes UDP probes on its tunnel address.
    TunListen,
    /// P1: dial a peer (address blob), bring up the /31 TUN tunnel, and probe
    /// reachability across it (UDP round-trip to the peer's link address).
    TunConnect {
        /// Address blob printed by the peer's `tun-listen`.
        addr: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // tun-test needs no iroh endpoint — handle it before binding one.
    if let Command::TunTest = cli.command {
        return run_tun_test().await;
    }

    // --lan implies no relay (LAN discovery only).
    let no_relay = cli.no_relay || cli.lan;
    let endpoint = build_endpoint(
        cli.secret_file.as_deref(),
        no_relay,
        cli.bind,
        cli.lan,
        &cli.relay,
    )
    .await?;

    match cli.command {
        Command::Id => {
            wait_until_addressable(&endpoint, no_relay).await;
            print_identity(&endpoint)?;
        }
        Command::Listen => run_listen(endpoint, no_relay).await?,
        Command::Connect { addr } => run_connect(endpoint, &addr).await?,
        Command::TunListen => run_tun_listen(endpoint, no_relay).await?,
        Command::TunConnect { addr } => run_tun_connect(endpoint, &addr).await?,
        Command::TunTest => unreachable!("handled above"),
    }
    Ok(())
}

/// A production [`DatagramConn`] over an iroh connection — the glue that lets the
/// substrate's TUN encap loops shuttle IP packets over iroh QUIC datagrams.
#[derive(Clone)]
struct IrohDatagramConn {
    conn: Connection,
}

#[async_trait::async_trait]
impl DatagramConn for IrohDatagramConn {
    async fn send_datagram(&self, packet: Bytes) -> Result<(), EncapError> {
        // Use the *waiting* send: under congestion (notably right after connect,
        // when the congestion window is tiny and we may still be relay-only), the
        // non-waiting `send_datagram` silently drops datagrams oldest-first. That
        // is the wrong policy for an L3 data plane — it turns transient backpressure
        // into packet loss the upper layers must recover from. `send_datagram_wait`
        // instead applies backpressure to the TUN reader until buffer space frees.
        let len = packet.len();
        self.conn.send_datagram_wait(packet).await.map_err(|e| {
            use iroh::endpoint::SendDatagramError;
            match e {
                SendDatagramError::TooLarge => EncapError::DatagramTooLarge(len),
                other => EncapError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    other.to_string(),
                )),
            }
        })
    }

    async fn recv_datagram(&self) -> Result<Bytes, EncapError> {
        // Any read error means the connection is no longer usable; surface it as
        // ConnectionClosed so the encap loop exits cleanly.
        self.conn
            .read_datagram()
            .await
            .map_err(|_| EncapError::ConnectionClosed)
    }
}

/// Short peer id for the interface name (8 hex chars is unique enough).
fn short_id(id: &str) -> &str {
    &id[..id.len().min(8)]
}

/// P1 listener: accept tunnel connections, bring up a /31 TUN per peer.
async fn run_tun_listen(endpoint: Endpoint, no_relay: bool) -> Result<()> {
    wait_until_addressable(&endpoint, no_relay).await;
    print_identity(&endpoint)?;
    info!("tun-listen: hand the address above to a peer's `tun-connect`");

    let self_id = endpoint.id().to_string();
    let router = Router::builder(endpoint)
        .accept(TUN_ALPN, TunnelHandler { self_id })
        .spawn();

    tokio::signal::ctrl_c().await.context("waiting for Ctrl-C")?;
    router.shutdown().await.context("router shutdown")?;
    Ok(())
}

/// P1 connector: dial a peer, bring up the tunnel, probe reachability across it.
async fn run_tun_connect(endpoint: Endpoint, addr_blob: &str) -> Result<()> {
    let addr = parse_peer(addr_blob).context("parsing peer")?;
    let peer = addr.id;
    let self_id = endpoint.id().to_string();

    info!(%peer, "tun-connect: dialing");
    let conn = endpoint
        .connect(addr, TUN_ALPN)
        .await
        .context("connect failed")?;

    let (self_addr, peer_addr) = mjolnir_mesh::tun::pick_link_31(&self_id, &peer.to_string());
    let tunnel = spawn_tunnel(
        short_id(&peer.to_string()),
        self_addr,
        peer_addr,
        IrohDatagramConn { conn: conn.clone() },
    )
    .await
    .context("bringing up tunnel")?;

    info!(
        iface = %tunnel.iface_name, %self_addr, %peer_addr,
        "tunnel up — probing reachability across it"
    );
    // Echo server on our own link addr (so the peer can probe us too).
    spawn_udp_echo(self_addr);
    // Give the peer a moment to bring up its side. iroh returns from connect()
    // as soon as a QUIC connection exists — which is over the *relay* initially;
    // hole-punching to a direct path happens asynchronously over the next few
    // seconds. Probing inside that window measures relay-only loss, which is high
    // for unreliable datagrams. Wait (bounded) for a direct path before the
    // headline probe, then report which path actually carried it.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let direct = wait_for_direct_path(&conn, Duration::from_secs(10)).await;
    log_conn_paths(&conn);
    probe_peer(peer_addr, direct).await;

    info!("tunnel established; holding open (Ctrl-C to exit)");
    tokio::signal::ctrl_c().await.context("waiting for Ctrl-C")?;
    drop(tunnel);
    Ok(())
}

/// iroh protocol handler that brings up a per-peer TUN tunnel on accept.
#[derive(Clone, Debug)]
struct TunnelHandler {
    self_id: String,
}

impl ProtocolHandler for TunnelHandler {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        let peer = conn.remote_id();
        let peer_str = peer.to_string();
        let (self_addr, peer_addr) = mjolnir_mesh::tun::pick_link_31(&self.self_id, &peer_str);

        match spawn_tunnel(
            short_id(&peer_str),
            self_addr,
            peer_addr,
            IrohDatagramConn { conn: conn.clone() },
        )
        .await
        {
            Ok(tunnel) => {
                info!(iface = %tunnel.iface_name, %self_addr, %peer_addr, %peer, "tunnel up (accepted)");
                spawn_udp_echo(self_addr);
                // Hold the tunnel open until the connection closes.
                let reason = conn.closed().await;
                info!(%peer, ?reason, "tunnel connection closed");
                drop(tunnel);
            }
            Err(e) => {
                warn!(%peer, "failed to bring up tunnel: {e}");
                conn.close(1u32.into(), b"tunnel setup failed");
            }
        }
        Ok(())
    }
}

/// Echo any UDP datagram back to its sender, bound to `bind_ip:TUN_PROBE_PORT`
/// (the TUN /31 address). Lets a peer prove the tunnel carries real IP traffic.
fn spawn_udp_echo(bind_ip: Ipv4Addr) {
    tokio::spawn(async move {
        let sock = match tokio::net::UdpSocket::bind((bind_ip, TUN_PROBE_PORT)).await {
            Ok(s) => s,
            Err(e) => {
                warn!(%bind_ip, "udp echo bind failed: {e}");
                return;
            }
        };
        info!(%bind_ip, port = TUN_PROBE_PORT, "udp echo up on tunnel address");
        let mut buf = [0u8; 1500];
        loop {
            match sock.recv_from(&mut buf).await {
                Ok((n, from)) => {
                    let _ = sock.send_to(&buf[..n], from).await;
                }
                Err(e) => {
                    warn!("udp echo recv error: {e}");
                    break;
                }
            }
        }
    });
}

/// Wait (bounded) for the connection to acquire a direct (hole-punched) path in
/// addition to the relay. Returns `true` if a direct path was established within
/// `timeout`, `false` if it stayed relay-only. A relay-only path forwards
/// unreliable datagrams best-effort and drops heavily under load, so the data
/// plane is far lossier before this returns true.
async fn wait_for_direct_path(conn: &Connection, timeout: Duration) -> bool {
    // Poll path snapshots rather than the path stream: the stream needs
    // `StreamExt` (futures-util), which is a Linux-only dep here, whereas
    // `paths()` is a plain snapshot that works on every platform.
    let deadline = Instant::now() + timeout;
    loop {
        if conn.paths().iter().any(|p| p.is_ip()) {
            return true;
        }
        if Instant::now() >= deadline {
            warn!(
                ?timeout,
                "no direct path within timeout — still relay-only; datagram loss \
                 will be high until a hole-punch succeeds"
            );
            return false;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Log a one-line summary of every QUIC path on the connection (relay vs direct,
/// selected, RTT) plus the current datagram-size ceiling. This is the diagnostic
/// that turns a bare "1/5 probes crossed" into "1/5 on a relay-only path".
fn log_conn_paths(conn: &Connection) {
    let paths = conn.paths();
    for p in paths.iter() {
        let kind = if p.is_relay() { "relay" } else { "direct" };
        info!(
            kind,
            selected = p.is_selected(),
            remote = %p.remote_addr(),
            rtt = ?p.rtt(),
            "tunnel path"
        );
    }
    info!(
        max_datagram_size = ?conn.max_datagram_size(),
        path_count = paths.len(),
        "tunnel connection datagram ceiling"
    );
}

/// Send a few UDP probes to `peer_ip:TUN_PROBE_PORT` over the tunnel and report
/// round-trip results. Success proves real IP traffic flows across the mesh.
/// `direct_path` records whether a hole-punched path was up, so the headline
/// makes relay-only loss legible rather than mysterious.
async fn probe_peer(peer_ip: Ipv4Addr, direct_path: bool) {
    let sock = match tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await {
        Ok(s) => s,
        Err(e) => {
            warn!("probe socket bind failed: {e}");
            return;
        }
    };
    let mut ok = 0u32;
    for i in 1..=5u32 {
        let payload = format!("mjolnir-tun-ping-{i}");
        let start = Instant::now();
        if let Err(e) = sock.send_to(payload.as_bytes(), (peer_ip, TUN_PROBE_PORT)).await {
            warn!("probe {i} send failed: {e}");
            continue;
        }
        let mut buf = [0u8; 256];
        match tokio::time::timeout(Duration::from_secs(2), sock.recv_from(&mut buf)).await {
            Ok(Ok((n, _))) if &buf[..n] == payload.as_bytes() => {
                ok += 1;
                println!("tunnel ping {i}: reply from {peer_ip} in {:?}", start.elapsed());
            }
            Ok(Ok((n, _))) => println!("tunnel ping {i}: unexpected {n}-byte reply"),
            Ok(Err(e)) => warn!("probe {i} recv error: {e}"),
            Err(_) => println!("tunnel ping {i}: TIMEOUT (no reply across tunnel)"),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let path = if direct_path { "direct path" } else { "RELAY-ONLY path (lossy)" };
    println!(
        "tunnel reachability: {ok}/5 replies over {path} — {}",
        if ok > 0 { "DATA PLANE WORKS" } else { "no traffic crossed" }
    );
}

/// Probe TUN-device creation — the gating check for running the L3 data plane
/// inside a RouterOS container (needs /dev/net/tun + CAP_NET_ADMIN).
async fn run_tun_test() -> Result<()> {
    use mjolnir_mesh::tun::PeerInterface;
    use std::net::Ipv4Addr;

    // Throwaway /31 in the reserved link block.
    let self_addr = Ipv4Addr::new(10, 255, 0, 0);
    let peer_addr = Ipv4Addr::new(10, 255, 0, 1);

    info!("tun-test: attempting to create a TUN device…");
    match PeerInterface::create("tuntest0", self_addr, peer_addr).await {
        Ok(iface) => {
            println!(
                "TUN OK: created {} ({} <-> {})",
                iface.name(),
                iface.self_addr(),
                iface.peer_addr()
            );
            match iface.close().await {
                Ok(()) => println!("TUN teardown OK — the L3 data plane is viable here"),
                Err(e) => println!("TUN created but teardown failed: {e}"),
            }
            Ok(())
        }
        Err(e) => {
            println!("TUN FAILED: {e}");
            anyhow::bail!("tun-test failed: {e}")
        }
    }
}

/// Build an iroh endpoint with a persisted (or ephemeral) identity. Relays are
/// on by default (they provide NAT traversal off-LAN); `--no-relay` forces
/// direct/LAN-only, and `--bind` pins the socket address.
async fn build_endpoint(
    secret_file: Option<&Path>,
    no_relay: bool,
    bind: Option<SocketAddr>,
    lan: bool,
    relays: &[String],
) -> Result<Endpoint> {
    let secret = load_or_create_secret(secret_file)?;

    if lan {
        // LAN-direct: start from the Minimal preset (crypto provider only, no
        // pkarr/n0-DNS publishing, so no internet dependency and no DNS spam),
        // relays off, and add ONLY mDNS address lookup for same-network peers.
        let mut builder = Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Disabled)
            .secret_key(secret)
            .address_lookup(MdnsAddressLookup::builder());
        if let Some(addr) = bind {
            builder = builder.bind_addr(addr).context("invalid --bind address")?;
        }
        return builder.bind().await.context("failed to bind iroh endpoint");
    }

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
        // iroh 0.96's RelayMode::Default points at the flaky `iroh-canary` test
        // network; Staging uses real n0 relays on relay.iroh.network.
        RelayMode::Staging
    };

    // N0 preset: publish to pkarr + resolve via n0 DNS (the internet path);
    // relay_mode below overrides the preset's default relay choice.
    let mut builder = Endpoint::builder(presets::N0)
        .secret_key(secret)
        .relay_mode(relay_mode);
    if let Some(addr) = bind {
        builder = builder.bind_addr(addr).context("invalid --bind address")?;
    }
    builder.bind().await.context("failed to bind iroh endpoint")
}

/// Wait until the endpoint has at least one publishable address. With relays
/// on, also wait for the home relay so the blob is dialable off-LAN.
async fn wait_until_addressable(endpoint: &Endpoint, no_relay: bool) {
    if !no_relay {
        // home-relay handshake; bounded so we don't hang forever if relays are
        // unreachable (e.g. offline) — direct addrs may still suffice.
        let _ = tokio::time::timeout(Duration::from_secs(5), endpoint.online()).await;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while endpoint.addr().is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    info!(addr = ?endpoint.addr(), "endpoint addressable");
    check_reachability(endpoint, no_relay);
}

/// One loud, actionable line about whether this node is reachable by peers —
/// instead of leaving the operator to infer it from buried pkarr/DNS spam.
/// A node with no relay and only private/loopback addresses has an unroutable
/// address blob (the classic "container has no internet egress" failure).
fn check_reachability(endpoint: &Endpoint, no_relay: bool) {
    let addr = endpoint.addr();
    let has_relay = addr.relay_urls().next().is_some();
    let ips: Vec<IpAddr> = addr.ip_addrs().map(|sa| sa.ip()).collect();
    let has_public = ips.iter().any(|ip| is_globally_reachable(*ip));
    let has_nonloopback = ips.iter().any(|ip| !ip.is_loopback());

    if has_relay || has_public {
        info!(relay = has_relay, public_ip = has_public, "reachability OK — peers can connect");
    } else if no_relay && has_nonloopback {
        warn!(
            "--no-relay: only private/LAN addresses — reachable on the LOCAL network only, \
             not across NATs. Fine for a same-LAN test; useless for a real swarm peer."
        );
    } else {
        error!(
            "NOT REACHABLE: no iroh relay and no public address. Peers on other networks \
             CANNOT connect to this node and its address blob is UNROUTABLE. Almost always the \
             container has no internet egress — check, in order: (1) veth `gateway=` / default \
             route, (2) NAT masquerade for the container subnet, (3) a firewall forward 'accept' \
             rule for that subnet, (4) the container `dns=` setting. The router itself having \
             internet is not enough — the *container's* forwarded traffic must reach the internet."
        );
    }
}

/// Is `ip` routable from outside the local network (i.e. usable in a blob a
/// remote peer could dial)?
fn is_globally_reachable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified())
        }
        IpAddr::V6(v6) => !(v6.is_loopback() || v6.is_unspecified()),
    }
}

fn print_identity(endpoint: &Endpoint) -> Result<()> {
    println!("node id: {}", endpoint.id());
    println!("address: {}", encode_addr(&endpoint.addr())?);
    Ok(())
}

async fn run_listen(endpoint: Endpoint, no_relay: bool) -> Result<()> {
    wait_until_addressable(&endpoint, no_relay).await;
    print_identity(&endpoint)?;
    info!(
        alpn = %String::from_utf8_lossy(MESH_ALPN),
        "listening — hand the address above to `connect`"
    );

    let router = Router::builder(endpoint)
        .accept(MESH_ALPN, PingHandler)
        .spawn();

    tokio::signal::ctrl_c().await.context("waiting for Ctrl-C")?;
    info!("shutting down");
    router.shutdown().await.context("router shutdown")?;
    Ok(())
}

async fn run_connect(endpoint: Endpoint, addr_blob: &str) -> Result<()> {
    let addr = parse_peer(addr_blob).context("parsing peer")?;
    let peer = addr.id;
    info!(%peer, "dialing");

    let conn = endpoint
        .connect(addr, MESH_ALPN)
        .await
        .context("connect failed")?;
    info!(%peer, "connection established");

    let payload = Bytes::from_static(PING);
    let start = Instant::now();
    conn.send_datagram(payload.clone())
        .context("send_datagram failed")?;
    let echoed = conn.read_datagram().await.context("no echo received")?;
    let rtt = start.elapsed();

    if echoed == payload {
        println!("round-trip OK to {peer} in {rtt:?}");
    } else {
        println!("echo MISMATCH from {peer} ({} bytes back)", echoed.len());
    }

    conn.close(0u32.into(), b"done");
    Ok(())
}

/// iroh protocol handler that echoes every datagram back to the sender until
/// the connection closes. The P0 "shuttle packets" stand-in.
#[derive(Debug, Clone)]
struct PingHandler;

impl ProtocolHandler for PingHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let peer = connection.remote_id();
        info!(%peer, "inbound mesh connection");
        loop {
            match connection.read_datagram().await {
                Ok(dg) => {
                    if let Err(e) = connection.send_datagram(dg) {
                        warn!(%peer, "echo failed: {e}");
                        break;
                    }
                }
                Err(e) => {
                    info!(%peer, "connection ended: {e}");
                    break;
                }
            }
        }
        Ok(())
    }
}

// --- identity persistence -------------------------------------------------

fn load_or_create_secret(path: Option<&Path>) -> Result<SecretKey> {
    if let Some(p) = path {
        if p.exists() {
            let hex = std::fs::read_to_string(p)
                .with_context(|| format!("reading secret file {}", p.display()))?;
            return parse_secret_hex(hex.trim());
        }
        let secret = SecretKey::generate();
        std::fs::write(p, encode_secret_hex(&secret))
            .with_context(|| format!("writing secret file {}", p.display()))?;
        info!(path = %p.display(), id = %secret.public(), "generated new node identity");
        return Ok(secret);
    }

    if let Ok(env) = std::env::var("IROH_SECRET") {
        return env.parse::<SecretKey>().context("parsing IROH_SECRET");
    }

    warn!("no --secret-file or IROH_SECRET set; using an ephemeral identity");
    Ok(SecretKey::generate())
}

fn encode_secret_hex(secret: &SecretKey) -> String {
    data_encoding::HEXLOWER.encode(&secret.to_bytes())
}

fn parse_secret_hex(hex: &str) -> Result<SecretKey> {
    let bytes = data_encoding::HEXLOWER
        .decode(hex.as_bytes())
        .context("secret file is not valid lowercase hex")?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .context("secret must be exactly 32 bytes")?;
    Ok(SecretKey::from_bytes(&arr))
}

// --- address blob (postcard + base32, matching the node's ticket scheme) ---

fn encode_addr(addr: &EndpointAddr) -> Result<String> {
    let bytes = postcard::to_allocvec(addr).context("serializing address")?;
    Ok(data_encoding::BASE32_NOPAD.encode(&bytes).to_lowercase())
}

fn decode_addr(blob: &str) -> Result<EndpointAddr> {
    let bytes = data_encoding::BASE32_NOPAD
        .decode(blob.to_uppercase().as_bytes())
        .context("address blob is not valid base32")?;
    postcard::from_bytes(&bytes).context("deserializing address")
}

/// Accept either a full address blob, or a bare 64-hex node id (whose address
/// is resolved via discovery — e.g. mDNS in `--lan` mode).
fn parse_peer(arg: &str) -> Result<EndpointAddr> {
    if arg.len() == 64 && arg.bytes().all(|b| b.is_ascii_hexdigit()) {
        let id: EndpointId = arg.parse().context("parsing node id")?;
        Ok(EndpointAddr::new(id))
    } else {
        decode_addr(arg)
    }
}
