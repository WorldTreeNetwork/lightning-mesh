// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! App `/128` for the selected house, and the Linux TUN runner.
//!
//! The guild surface only computes the address. `open_app_tun` is called by a
//! test or a second process that already holds one `DatagramConn`.

use std::net::Ipv6Addr;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use mjolnir_mesh::tun::{
    app_ula, house_addr_usable, parse_house_guild_id, spawn_encap_pair, DatagramConn,
};

const MTU: usize = 1280;

/// Address to show for the guild the existing surface has selected.
///
/// A workplace house has no outer guild. A nested access guild does.
/// Missing inputs yield nothing. This function opens no TUN and writes no route.
pub fn display_app_address(
    outer: Option<&str>,
    guild_hex: &str,
    actor_public_hex: Option<&str>,
) -> Option<String> {
    if outer.is_some() {
        return None;
    }
    let guild = parse_house_guild_id(guild_hex)?;
    let actor = parse_house_guild_id(actor_public_hex?)?;
    Some(app_ula(&guild, &actor).to_string())
}

/// Result of `open_app_tun`. Drop removes the address, the peer route, and the device.
pub struct AppTunOpen {
    pub address: Option<String>,
    pub up: bool,
    inner: Option<Session>,
}

impl AppTunOpen {
    pub fn is_up(&self) -> bool {
        self.inner
            .as_ref()
            .map(|s| s.up.load(Ordering::SeqCst))
            .unwrap_or(false)
            && self.up
    }
}

struct Session {
    name: String,
    local: Ipv6Addr,
    peer: Ipv6Addr,
    up: Arc<AtomicBool>,
    watch: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(watch) = self.watch.take() {
            watch.abort();
        }
        remove_owned(&self.name, self.local, self.peer);
        self.up.store(false, Ordering::SeqCst);
    }
}

fn down(address: Option<String>) -> AppTunOpen {
    AppTunOpen {
        address,
        up: false,
        inner: None,
    }
}

/// Open one TUN at the caller's app `/128` and pass packets to `conn`.
///
/// `bound_node_id` is the node id the caller already bound to `conn`. Any other
/// id installs nothing and does not use `conn`.
pub async fn open_app_tun<C>(
    guild_id: [u8; 32],
    actor_public: [u8; 32],
    peer_node_id: [u8; 32],
    bound_node_id: [u8; 32],
    peer_addr: Ipv6Addr,
    conn: C,
) -> AppTunOpen
where
    C: DatagramConn + Clone,
{
    let local = app_ula(&guild_id, &actor_public);
    let address = Some(local.to_string());
    if peer_node_id != bound_node_id {
        return down(address);
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (peer_addr, conn);
        return down(address);
    }
    #[cfg(target_os = "linux")]
    {
        open_linux(local, peer_addr, conn, address).await
    }
}

#[cfg(target_os = "linux")]
async fn open_linux<C>(
    local: Ipv6Addr,
    peer: Ipv6Addr,
    conn: C,
    address: Option<String>,
) -> AppTunOpen
where
    C: DatagramConn + Clone,
{
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = format!("mja{n:x}");
    let mut config = tun::Configuration::default();
    config.tun_name(&name).mtu(MTU as u16).up();
    let dev = match tun::create_as_async(&config) {
        Ok(dev) => dev,
        Err(_) => return down(address),
    };
    if ip(&["link", "set", &name, "up"]).is_err()
        || ip(&[
            "-6",
            "addr",
            "replace",
            &format!("{local}/128"),
            "dev",
            &name,
            "nodad",
        ])
        .is_err()
    {
        let _ = ip(&["link", "del", &name]);
        return down(address);
    }
    let usable = std::fs::read_to_string("/proc/net/if_inet6")
        .ok()
        .is_some_and(|body| house_addr_usable(&body, &name, local));
    if !usable
        || ip(&[
            "-6",
            "route",
            "replace",
            &format!("{peer}/128"),
            "dev",
            &name,
        ])
        .is_err()
    {
        remove_owned(&name, local, peer);
        return down(address);
    }
    let (write, read) = match dev.split() {
        Ok(halves) => halves,
        Err(_) => {
            remove_owned(&name, local, peer);
            return down(address);
        }
    };
    let handles = spawn_encap_pair(read, write, conn, MTU);
    let up = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&up);
    let dev_name = name.clone();
    let watch = tokio::spawn(async move {
        let mut tx = handles.tx;
        let mut rx = handles.rx;
        tokio::select! {
            _ = &mut tx => {}
            _ = &mut rx => {}
        }
        drop(tx);
        drop(rx);
        remove_owned(&dev_name, local, peer);
        flag.store(false, Ordering::SeqCst);
    });
    AppTunOpen {
        address,
        up: true,
        inner: Some(Session {
            name,
            local,
            peer,
            up,
            watch: Some(watch),
        }),
    }
}

fn ip(args: &[&str]) -> Result<(), ()> {
    if cfg!(test)
        && args.get(1) == Some(&"route")
        && args.get(2) == Some(&"replace")
        && std::env::var("MJ_APP_ROUTE_FAIL").ok().as_deref() == Some("1")
    {
        return Err(());
    }
    let ok = Command::new("ip")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(())
    }
}

fn remove_owned(name: &str, local: Ipv6Addr, peer: Ipv6Addr) {
    let _ = ip(&["-6", "route", "del", &format!("{peer}/128"), "dev", name]);
    let _ = ip(&["-6", "addr", "del", &format!("{local}/128"), "dev", name]);
    let _ = ip(&["link", "del", name]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bytes::Bytes;
    use mjolnir_mesh::tun::EncapError;
    use tokio::sync::{mpsc, Mutex};

    fn fixture_addr(guild: u8, key: u8) -> Ipv6Addr {
        app_ula(&[guild; 32], &[key; 32])
    }

    #[test]
    fn fixture_app_address_matches_the_house_vector() {
        let shown = display_app_address(None, &"11".repeat(32), Some(&"ab".repeat(32))).unwrap();
        assert_eq!(shown, "fd81:2b22:8aca:1:bd5e:fa7a:5460:697");
        assert_eq!(fixture_addr(0x11, 0xab).to_string(), shown);
    }

    #[test]
    fn missing_actor_or_nested_guild_shows_nothing() {
        assert!(display_app_address(None, &"11".repeat(32), None).is_none());
        assert!(display_app_address(
            Some("22".repeat(32).as_str()),
            &"11".repeat(32),
            Some(&"ab".repeat(32))
        )
        .is_none());
    }

    #[test]
    fn workplace_house_shows_its_own_address() {
        let shown = display_app_address(None, &"22".repeat(32), Some(&"ab".repeat(32))).unwrap();
        assert_eq!(shown, fixture_addr(0x22, 0xab).to_string());
        assert_ne!(shown, "fd81:2b22:8aca:1:bd5e:fa7a:5460:697");
    }

    #[derive(Clone)]
    struct Silent {
        tx: mpsc::Sender<Bytes>,
        rx: Arc<Mutex<mpsc::Receiver<Bytes>>>,
        sent: Arc<Mutex<Vec<Bytes>>>,
    }

    #[async_trait]
    impl DatagramConn for Silent {
        async fn send_datagram(&self, packet: Bytes) -> Result<(), EncapError> {
            self.sent.lock().await.push(packet.clone());
            self.tx
                .send(packet)
                .await
                .map_err(|_| EncapError::ConnectionClosed)
        }
        async fn recv_datagram(&self) -> Result<Bytes, EncapError> {
            self.rx
                .lock()
                .await
                .recv()
                .await
                .ok_or(EncapError::ConnectionClosed)
        }
    }

    fn ends() -> (Silent, Silent) {
        let (a_tx, b_rx) = mpsc::channel(8);
        let (b_tx, a_rx) = mpsc::channel(8);
        let a = Silent {
            tx: a_tx,
            rx: Arc::new(Mutex::new(a_rx)),
            sent: Arc::new(Mutex::new(Vec::new())),
        };
        let b = Silent {
            tx: b_tx,
            rx: Arc::new(Mutex::new(b_rx)),
            sent: Arc::new(Mutex::new(Vec::new())),
        };
        (a, b)
    }

    #[tokio::test]
    async fn unknown_peer_installs_nothing() {
        let (conn, _other) = ends();
        let before = std::fs::read_dir("/sys/class/net")
            .map(|d| d.count())
            .unwrap_or(0);
        let opened = open_app_tun(
            [0x11; 32],
            [0xab; 32],
            [0x22; 32],
            [0x33; 32],
            fixture_addr(0x11, 0xcd),
            conn,
        )
        .await;
        let after = std::fs::read_dir("/sys/class/net")
            .map(|d| d.count())
            .unwrap_or(0);
        assert_eq!(
            opened.address.as_deref(),
            Some("fd81:2b22:8aca:1:bd5e:fa7a:5460:697")
        );
        assert!(!opened.is_up());
        assert_eq!(before, after);
        assert!(_other.sent.lock().await.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn route_install_failure_removes_the_address() {
        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        let local = fixture_addr(0x11, 0xab);
        let (conn, _other) = ends();
        std::env::set_var("MJ_APP_ROUTE_FAIL", "1");
        let opened = open_app_tun(
            [0x11; 32],
            [0xab; 32],
            [0x22; 32],
            [0x22; 32],
            fixture_addr(0x11, 0xcd),
            conn,
        )
        .await;
        std::env::remove_var("MJ_APP_ROUTE_FAIL");
        assert_eq!(opened.address.as_deref(), Some(local.to_string().as_str()));
        assert!(!opened.is_up());
        let body = std::fs::read_to_string("/proc/net/if_inet6").unwrap_or_default();
        let hex: String = local.octets().iter().map(|b| format!("{b:02x}")).collect();
        assert!(!body.to_ascii_lowercase().contains(&hex));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn packet_crosses_the_labeled_conn() {
        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        let local = fixture_addr(0x11, 0xab);
        let peer = fixture_addr(0x11, 0xcd);
        let mut peer_cfg = tun::Configuration::default();
        peer_cfg.tun_name("mjpeer").mtu(1280).up();
        let peer_dev = tun::create_as_async(&peer_cfg).expect("peer tun");
        let _ = ip(&["netns", "del", "mjapp-peer"]);
        ip(&["netns", "add", "mjapp-peer"]).expect("netns");
        ip(&["link", "set", "mjpeer", "netns", "mjapp-peer"]).expect("move");
        ip(&[
            "netns",
            "exec",
            "mjapp-peer",
            "ip",
            "link",
            "set",
            "mjpeer",
            "up",
        ])
        .unwrap();
        ip(&[
            "netns",
            "exec",
            "mjapp-peer",
            "ip",
            "-6",
            "addr",
            "add",
            &format!("{peer}/128"),
            "dev",
            "mjpeer",
            "nodad",
        ])
        .unwrap();
        ip(&[
            "netns",
            "exec",
            "mjapp-peer",
            "ip",
            "-6",
            "route",
            "add",
            &format!("{local}/128"),
            "dev",
            "mjpeer",
        ])
        .unwrap();
        let (write_p, read_p) = peer_dev.split().unwrap();
        let (conn_local, conn_peer) = ends();
        let sent = Arc::clone(&conn_local.sent);
        let peer_handles = spawn_encap_pair(read_p, write_p, conn_peer, MTU);
        let opened = open_app_tun(
            [0x11; 32], [0xab; 32], [0x22; 32], [0x22; 32], peer, conn_local,
        )
        .await;
        assert_eq!(opened.address.as_deref(), Some(local.to_string().as_str()));
        assert!(opened.is_up(), "session did not come up");
        let ping = Command::new("ping")
            .args(["-6", "-c", "1", "-W", "2", &peer.to_string()])
            .status();
        let packets = sent.lock().await.clone();
        let saw_request = packets.iter().any(|p| {
            p.first().is_some_and(|v| v >> 4 == 6)
                && p.get(6) == Some(&58)
                && p.get(40) == Some(&128)
                && p.get(24..40) == Some(peer.octets().as_slice())
        });
        assert!(ping.unwrap().success(), "ping failed");
        assert!(saw_request, "echo request was not on the labeled conn");
        peer_handles.abort();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while opened.is_up() && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(!opened.is_up(), "conn close left the session up");
        let body = std::fs::read_to_string("/proc/net/if_inet6").unwrap_or_default();
        let hex: String = local.octets().iter().map(|b| format!("{b:02x}")).collect();
        assert!(
            !body.to_ascii_lowercase().contains(&hex),
            "address left behind after the conn closed"
        );
        drop(opened);
        let _ = ip(&["netns", "del", "mjapp-peer"]);
    }
}
