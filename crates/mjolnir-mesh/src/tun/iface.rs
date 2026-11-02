use std::net::{Ipv4Addr, Ipv6Addr};

use crate::tun::encap::DatagramConn;
#[cfg(target_os = "linux")]
use crate::tun::encap::{EncapHandles, spawn_encap_pair};

/// MTU for tunnel TUN devices. Kept under the QUIC datagram limit so one IP
/// packet fits in one iroh datagram (otherwise the encap drops it as
/// `EncapError::DatagramTooLarge`). Tunable; conservative for typical paths.
pub const TUNNEL_MTU: u16 = 1300;

/// Default name of the single overlay TUN (mjolnir-mesh-buw): one interface per
/// node multiplexing every peer, replacing the per-peer `mj-peer-*` tunnels.
pub const OVERLAY_IFACE: &str = "mjolnir0";

pub use super::neighbor::overlay_link_local;

/// A live single overlay TUN (`mjolnir0`). The [`tun::AsyncDevice`] is returned
/// separately and owns the fd — dropping it removes the interface. This struct
/// is just the addressing metadata the data plane and babeld renderer need.
#[derive(Debug, Clone)]
pub struct OverlayLink {
    pub iface_name: String,
    /// Identity used to derive this interface's sole link-local.
    pub node_id: String,
    /// The `fe80::` babeld runs its protocol over (see [`overlay_link_local`]).
    pub link_local: Ipv6Addr,
    /// Kernel interface index.
    pub index: u32,
}

/// A live per-peer L3 tunnel: a TUN interface plus the two encap tasks bridging
/// it to an iroh connection. Aborting the tasks (on drop) releases the TUN fd,
/// so the kernel removes the interface — no explicit teardown needed.
pub struct Tunnel {
    pub iface_name: String,
    pub self_addr: Ipv4Addr,
    pub peer_addr: Ipv4Addr,
    #[cfg(target_os = "linux")]
    handles: EncapHandles,
}

#[cfg(target_os = "linux")]
impl Drop for Tunnel {
    fn drop(&mut self) {
        self.handles.abort();
    }
}

/// Bring up a per-peer /31 TUN interface and bridge it to `conn` (an iroh
/// connection, via the [`DatagramConn`] seam). Returns once the interface is up
/// and the encap loops are running; the [`Tunnel`] keeps them alive until dropped.
#[cfg(target_os = "linux")]
pub async fn spawn_tunnel<C>(
    peer_short_id: &str,
    self_addr: Ipv4Addr,
    peer_addr: Ipv4Addr,
    conn: C,
) -> Result<Tunnel, IfaceError>
where
    C: DatagramConn + Clone,
{
    use futures_util::stream::TryStreamExt;
    use rtnetlink::new_connection;

    let raw_name = format!("mj-peer-{peer_short_id}");
    let iface_name: String = raw_name.chars().take(15).collect();

    // 1. Create the async TUN device (retained for the tunnel's lifetime), MTU set.
    let mut config = tun::Configuration::default();
    config.tun_name(&iface_name).mtu(TUNNEL_MTU).up();
    let device = tun::create_as_async(&config).map_err(|e| std::io::Error::other(e.to_string()))?;

    // 2. Assign self_addr/31 and bring the link up via rtnetlink.
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);
    let mut links = handle.link().get().match_name(iface_name.clone()).execute();
    let link = links.try_next().await?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("interface {iface_name} not found after creation"),
        )
    })?;
    let index = link.header.index;
    handle
        .address()
        .add(index, std::net::IpAddr::V4(self_addr), 31)
        .execute()
        .await?;
    handle
        .link()
        .set(rtnetlink::LinkUnspec::new_with_index(index).up().build())
        .execute()
        .await?;

    // 2b. Assign an IPv6 link-local so babeld can run its protocol over the
    //     tunnel — Babel transports over fe80:: (carrying both v4 and v6 routes),
    //     so a v4-only /31 gives babeld no neighbour and it installs no routes
    //     (mjolnir-mesh-op4). Distinct per side (derived from the /31 host octet)
    //     so the two ends are addressable to each other. Containers often
    //     default-disable IPv6, so enable it on the iface first. Best-effort: the
    //     v4 data plane works regardless, so failures here only degrade routing.
    let _ = std::fs::write(
        format!("/proc/sys/net/ipv6/conf/{iface_name}/disable_ipv6"),
        "0",
    );
    let link_local =
        std::net::Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, self_addr.octets()[3] as u16);
    if let Err(e) = handle
        .address()
        .add(index, std::net::IpAddr::V6(link_local), 64)
        .execute()
        .await
    {
        tracing::warn!(
            iface = %iface_name, %link_local,
            "could not assign IPv6 link-local (babeld adjacency may not form): {e}"
        );
    }

    // 3. Split the device and wire each half to the iroh connection.
    //    split() returns (writer, reader); encap wants (read, write).
    let (writer, reader) = device.split().map_err(IfaceError::Io)?;
    let handles = spawn_encap_pair(reader, writer, conn, TUNNEL_MTU as usize);

    Ok(Tunnel {
        iface_name,
        self_addr,
        peer_addr,
        handles,
    })
}

#[cfg(not(target_os = "linux"))]
pub async fn spawn_tunnel<C>(
    _peer_short_id: &str,
    _self_addr: Ipv4Addr,
    _peer_addr: Ipv4Addr,
    _conn: C,
) -> Result<Tunnel, IfaceError>
where
    C: DatagramConn + Clone,
{
    Err(IfaceError::Unsupported)
}

/// Bring up the single overlay TUN, with no IPv4 or house host assigned.
/// Its sole IPv6 link-local shares the router identity's hashed interface id.
#[cfg(target_os = "linux")]
pub async fn spawn_overlay_tun(
    node_id: &str,
    iface_name: &str,
) -> Result<(tun::AsyncDevice, OverlayLink), IfaceError> {
    use futures_util::stream::TryStreamExt;
    use rtnetlink::new_connection;
    use rtnetlink::packet_route::link::LinkFlags;

    let link_local = overlay_link_local(node_id);

    // 1. Create the async TUN device (retained for the node's lifetime), MTU set.
    let mut config = tun::Configuration::default();
    config.tun_name(iface_name).mtu(TUNNEL_MTU).up();
    let device = tun::create_as_async(&config).map_err(|e| std::io::Error::other(e.to_string()))?;

    // 2. Resolve the interface index.
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);
    let mut links = handle
        .link()
        .get()
        .match_name(iface_name.to_string())
        .execute();
    let link = links.try_next().await?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("interface {iface_name} not found after creation"),
        )
    })?;
    let index = link.header.index;

    // Disable automatic link-local generation before enabling IPv6. This TUN
    // has exactly one link-local, derived from the node id, and no IPv4/house IP.
    std::fs::write(
        format!("/proc/sys/net/ipv6/conf/{iface_name}/disable_ipv6"),
        "1",
    )?;
    std::fs::write(
        format!("/proc/sys/net/ipv6/conf/{iface_name}/addr_gen_mode"),
        "1",
    )?;
    std::fs::write(
        format!("/proc/sys/net/ipv6/conf/{iface_name}/disable_ipv6"),
        "0",
    )?;
    handle
        .address()
        .add(index, std::net::IpAddr::V6(link_local), 64)
        .execute()
        .await?;

    // 5. Bring the link up AND set the MULTICAST flag. A TUN has no MULTICAST flag
    //    by default; without it babeld silently never emits Hellos to ff02::1:6,
    //    so no neighbour is ever discovered (proven in mjolnir-mesh-buw.1).
    let mut msg = rtnetlink::LinkUnspec::new_with_index(index).up().build();
    msg.header.flags |= LinkFlags::Multicast;
    msg.header.change_mask |= LinkFlags::Multicast;
    handle.link().set(msg).execute().await?;

    let overlay = OverlayLink {
        iface_name: iface_name.to_string(),
        node_id: node_id.to_owned(),
        link_local,
        index,
    };
    tracing::info!(iface = %iface_name, %node_id, %link_local, "overlay TUN up");
    Ok((device, overlay))
}

// spawn_overlay_tun is Linux-only: the whole overlay data plane depends on the
// `tun` crate + rtnetlink, which are Linux-only deps. Callers (the daemon) are
// Linux-only too; no cross-platform stub is provided.

/// A live per-peer TUN interface. Drops the interface on `close()`.
pub struct PeerInterface {
    name: String,
    self_addr: Ipv4Addr,
    peer_addr: Ipv4Addr,
}

impl PeerInterface {
    /// Create the TUN interface, assign the /31, bring link up.
    ///
    /// `peer_short_id` is a short hex prefix of the peer's iroh NodeId
    /// (8 chars is enough for uniqueness in any realistic mesh).
    ///
    /// Linux-only for the MVP. On other platforms, returns
    /// `Err(IfaceError::Unsupported)` with a clear message.
    #[cfg(target_os = "linux")]
    pub async fn create(
        peer_short_id: &str,
        self_addr: Ipv4Addr,
        peer_addr: Ipv4Addr,
    ) -> Result<Self, IfaceError> {
        use futures_util::stream::TryStreamExt;
        use rtnetlink::new_connection;

        // Build interface name: "mj-peer-<peer_short_id>", truncated to 15 chars
        // (Linux IFNAMSIZ is 16 including the null terminator).
        let raw_name = format!("mj-peer-{peer_short_id}");
        let name: String = raw_name.chars().take(15).collect();

        // 1. Create the TUN device using the `tun` crate.
        let mut config = tun::Configuration::default();
        config.tun_name(&name).up();
        // We create the device but don't need the handle for this bead
        // (US-005 will use it for packet I/O). The device persists in the
        // kernel as long as we hold the tun::Device handle, so we keep it.
        let _device = tun::create(&config).map_err(|e| std::io::Error::other(e.to_string()))?;

        // 2. Use rtnetlink to assign the /31 address.
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);

        // Find the interface by name to get its index.
        let mut links = handle.link().get().match_name(name.clone()).execute();
        let link = links.try_next().await?.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("interface {name} not found after creation"),
            )
        })?;
        let index = link.header.index;

        // Add self_addr/31 to the interface.
        handle
            .address()
            .add(index, std::net::IpAddr::V4(self_addr), 31)
            .execute()
            .await?;

        // 3. Bring the link up.
        handle
            .link()
            .set(rtnetlink::LinkUnspec::new_with_index(index).up().build())
            .execute()
            .await?;

        Ok(Self {
            name,
            self_addr,
            peer_addr,
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn create(
        _peer_short_id: &str,
        _self_addr: Ipv4Addr,
        _peer_addr: Ipv4Addr,
    ) -> Result<Self, IfaceError> {
        Err(IfaceError::Unsupported)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn self_addr(&self) -> Ipv4Addr {
        self.self_addr
    }

    pub fn peer_addr(&self) -> Ipv4Addr {
        self.peer_addr
    }

    /// Tear down the interface. Idempotent: subsequent close() calls are no-ops.
    #[cfg(target_os = "linux")]
    pub async fn close(self) -> Result<(), IfaceError> {
        use futures_util::stream::TryStreamExt;
        use rtnetlink::new_connection;

        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);

        let mut links = handle.link().get().match_name(self.name.clone()).execute();
        if let Some(link) = links.try_next().await? {
            handle.link().del(link.header.index).execute().await?;
        }
        // If not found, it's already gone — idempotent.
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn close(self) -> Result<(), IfaceError> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IfaceError {
    #[cfg(target_os = "linux")]
    #[error("netlink error: {0}")]
    Netlink(#[from] rtnetlink::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported platform — TUN lifecycle is Linux-only")]
    Unsupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_link_local_is_stable_and_distinct() {
        let a = overlay_link_local("peer-example");
        assert_eq!(a.to_string(), "fe80::403d:2741:12c1:e5c0");
        assert_eq!(a, overlay_link_local("peer-example"));
        assert_ne!(a, overlay_link_local("node-example"));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[ignore = "requires root or CAP_NET_ADMIN; run with `cargo test -- --ignored`"]
    async fn overlay_tun_up_with_multicast_flag() {
        use futures_util::stream::TryStreamExt;
        use rtnetlink::packet_route::link::LinkFlags;

        let (device, link) = spawn_overlay_tun("node-example", "mjtest0")
            .await
            .expect("overlay TUN up");
        assert_eq!(link.iface_name, "mjtest0");
        assert_eq!(link.link_local, overlay_link_local("node-example"));

        // Verify the MULTICAST flag is actually set on the kernel interface —
        // the one thing a TUN lacks by default and babel needs (buw.1).
        let (conn, handle, _) = rtnetlink::new_connection().unwrap();
        tokio::spawn(conn);
        let msg = handle
            .link()
            .get()
            .match_index(link.index)
            .execute()
            .try_next()
            .await
            .unwrap()
            .expect("interface exists");
        assert!(
            msg.header.flags.contains(LinkFlags::Multicast),
            "overlay TUN must have IFF_MULTICAST set"
        );
        assert!(msg.header.flags.contains(LinkFlags::Up), "must be up");

        drop(device); // releases the fd -> kernel removes mjtest0
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[ignore = "requires root or CAP_NET_ADMIN; run with `cargo test -- --ignored`"]
    async fn create_and_destroy_real_tun() {
        let iface = PeerInterface::create(
            "test1234",
            "10.255.1.0".parse().unwrap(),
            "10.255.1.1".parse().unwrap(),
        )
        .await
        .expect("create succeeded");
        assert!(iface.name().starts_with("mj-peer-"));
        iface.close().await.expect("close succeeded");
    }

    #[cfg(not(target_os = "linux"))]
    #[tokio::test]
    async fn create_unsupported_off_linux() {
        let r = PeerInterface::create(
            "test1234",
            "10.255.1.0".parse().unwrap(),
            "10.255.1.1".parse().unwrap(),
        )
        .await;
        assert!(matches!(r, Err(IfaceError::Unsupported)));
    }
}
