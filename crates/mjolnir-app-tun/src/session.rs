// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::future::{Future, poll_fn};
use std::net::Ipv6Addr;
use std::pin::Pin;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use crate::{
    DatagramConn, EncapHandles, app_ula, house_addr_usable, house_ula_prefix, spawn_encap_pair,
};

const MTU: usize = 1280;

/// An app address and its optional live session. Drop cancels both packet tasks
/// and removes the address, peer route, and admitted routes owned by the session.
pub struct AppTunOpen {
    pub address: Option<String>,
    /// Whether opening succeeded. Use `is_up` for current task liveness.
    pub up: bool,
    inner: Option<Session>,
}

impl AppTunOpen {
    /// True only while both packet tasks are running on the configured TUN.
    pub fn is_up(&self) -> bool {
        self.up && self.inner.as_ref().is_some_and(Session::is_up)
    }

    /// The immutable house that opened the session, for caller admission policy.
    pub fn guild_id(&self) -> Option<[u8; 32]> {
        self.inner.as_ref().map(|s| s.owned.lock().unwrap().guild)
    }

    /// Reconcile already-admitted foreign `/48` prefix bytes. This crate does
    /// not authenticate admissions or read a guild log. Own-house prefixes are
    /// excluded using the guild stored at open. Failed deletes or observations
    /// retain ownership so a later sync can retry.
    pub fn sync_admitted_routes(&mut self, prefixes: &[[u8; 6]]) -> Result<(), ()> {
        let Some(session) = &self.inner else {
            return Ok(());
        };
        let mut owned = session.owned.lock().unwrap();
        if !session.is_up() {
            return Ok(());
        }
        owned.sync(prefixes)
    }
}

// A small command seam lets lifecycle tests observe address/route cleanup
// without root or a real TUN. Production always uses SystemNetwork.
trait Network: Send + Sync {
    fn ip(&self, args: &[&str]) -> Result<(), ()>;
    fn route_present(&self, dev: &str, prefix: &[u8; 6]) -> Result<bool, ()>;
}

struct SystemNetwork;
impl Network for SystemNetwork {
    fn ip(&self, args: &[&str]) -> Result<(), ()> {
        ip(args)
    }
    fn route_present(&self, dev: &str, prefix: &[u8; 6]) -> Result<bool, ()> {
        route_present(dev, prefix)
    }
}

struct Owned {
    name: String,
    local: Ipv6Addr,
    peer: Ipv6Addr,
    owned_prefixes: Vec<[u8; 6]>,
    guild: [u8; 32],
    network: Arc<dyn Network>,
}

impl Owned {
    fn delete_prefix(&self, prefix: &[u8; 6]) -> Result<(), ()> {
        self.network.ip(&[
            "-6",
            "route",
            "del",
            &format!("{}/48", prefix48(prefix)),
            "dev",
            &self.name,
        ])?;
        match self.network.route_present(&self.name, prefix) {
            Ok(false) => Ok(()),
            Ok(true) | Err(()) => Err(()),
        }
    }

    fn sync(&mut self, prefixes: &[[u8; 6]]) -> Result<(), ()> {
        let own = house_ula_prefix(&self.guild);
        let mut next = Vec::new();
        for prefix in prefixes {
            if prefix.as_slice() != &own.octets[..6] && !next.contains(prefix) {
                next.push(*prefix);
            }
        }
        let mut failed = false;
        let mut kept = Vec::new();
        for prefix in &self.owned_prefixes {
            if next.contains(prefix) || self.delete_prefix(prefix).is_err() {
                kept.push(*prefix);
                if !next.contains(prefix) {
                    failed = true;
                }
            }
        }
        for prefix in next {
            if kept.contains(&prefix) {
                continue;
            }
            if self
                .network
                .ip(&[
                    "-6",
                    "route",
                    "replace",
                    &format!("{}/48", prefix48(&prefix)),
                    "dev",
                    &self.name,
                ])
                .is_err()
            {
                failed = true;
                continue;
            }
            match self.network.route_present(&self.name, &prefix) {
                Ok(true) => kept.push(prefix),
                Ok(false) => failed = true,
                Err(()) => {
                    kept.push(prefix);
                    failed = true;
                }
            }
        }
        self.owned_prefixes = kept;
        if failed { Err(()) } else { Ok(()) }
    }

    fn cleanup(&mut self) {
        self.owned_prefixes = self
            .owned_prefixes
            .iter()
            .copied()
            .filter(|prefix| self.delete_prefix(prefix).is_err())
            .collect();
        remove_owned_with(self.network.as_ref(), &self.name, self.local, self.peer);
    }
}

struct Session {
    // The session retains the handles independently of the supervisor's future.
    // Dropping before that future polls must still cancel both children.
    handles: Arc<Mutex<EncapHandles>>,
    owned: Arc<Mutex<Owned>>,
    up: Arc<AtomicBool>,
    watch: tokio::task::JoinHandle<()>,
}

impl Session {
    fn start(owned: Owned, handles: EncapHandles) -> Self {
        let handles = Arc::new(Mutex::new(handles));
        let owned = Arc::new(Mutex::new(owned));
        let up = Arc::new(AtomicBool::new(true));
        let tasks = Arc::clone(&handles);
        let routes = Arc::clone(&owned);
        let flag = Arc::clone(&up);
        let watch = tokio::spawn(async move {
            // Poll the join handles under a short synchronous lock, never held
            // over an await. Only this supervisor awaits either handle.
            poll_fn(|cx| {
                let mut handles = tasks.lock().unwrap();
                if Pin::new(&mut handles.tx).poll(cx).is_ready()
                    || Pin::new(&mut handles.rx).poll(cx).is_ready()
                {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            tasks.lock().unwrap().abort();
            routes.lock().unwrap().cleanup();
            flag.store(false, Ordering::SeqCst);
        });
        Self {
            handles,
            owned,
            up,
            watch,
        }
    }

    fn is_up(&self) -> bool {
        let handles = self.handles.lock().unwrap();
        self.up.load(Ordering::SeqCst) && !handles.tx.is_finished() && !handles.rx.is_finished()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.handles.lock().unwrap().abort();
        self.watch.abort();
        self.owned.lock().unwrap().cleanup();
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
        open_linux(guild_id, local, peer_addr, conn, address).await
    }
}

#[cfg(target_os = "linux")]
async fn open_linux<C>(
    guild: [u8; 32],
    local: Ipv6Addr,
    peer: Ipv6Addr,
    conn: C,
    address: Option<String>,
) -> AppTunOpen
where
    C: DatagramConn + Clone,
{
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
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
    AppTunOpen {
        address,
        up: true,
        inner: Some(Session::start(
            Owned {
                name,
                local,
                peer,
                guild,
                owned_prefixes: Vec::new(),
                network: Arc::new(SystemNetwork),
            },
            handles,
        )),
    }
}

fn ip(args: &[&str]) -> Result<(), ()> {
    #[cfg(test)]
    if args.get(1) == Some(&"route")
        && matches!(args.get(2), Some(&"replace") | Some(&"del"))
        && std::env::var("MJ_APP_ROUTE_FAIL").ok().as_deref() == Some("1")
    {
        return Err(());
    }
    #[cfg(test)]
    if args.get(2) == Some(&"del")
        && std::env::var("MJ_APP_ROUTE_DEL_SKIP").ok().as_deref() == Some("1")
    {
        return Ok(());
    }
    let ok = Command::new("ip")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok { Ok(()) } else { Err(()) }
}

fn remove_owned(name: &str, local: Ipv6Addr, peer: Ipv6Addr) {
    remove_owned_with(&SystemNetwork, name, local, peer);
}

fn remove_owned_with(network: &dyn Network, name: &str, local: Ipv6Addr, peer: Ipv6Addr) {
    let _ = network.ip(&["-6", "route", "del", &format!("{peer}/128"), "dev", name]);
    let _ = network.ip(&["-6", "addr", "del", &format!("{local}/128"), "dev", name]);
    let _ = network.ip(&["link", "del", name]);
}

fn prefix48(prefix: &[u8; 6]) -> Ipv6Addr {
    let mut octets = [0u8; 16];
    octets[..6].copy_from_slice(prefix);
    Ipv6Addr::from(octets)
}

fn route_present(dev: &str, prefix: &[u8; 6]) -> Result<bool, ()> {
    #[cfg(test)]
    if std::env::var("MJ_APP_ROUTE_OBSERVE_FAIL").ok().as_deref() == Some("1") {
        return Err(());
    }
    let text = Command::new("ip")
        .args(["-6", "route", "show", "dev", dev])
        .output()
        .map_err(|_| ())?;
    if !text.status.success() {
        return Err(());
    }
    let body = String::from_utf8_lossy(&text.stdout);
    Ok(body
        .lines()
        .any(|line| line.starts_with(&format!("{}/48", prefix48(prefix)))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EncapError;
    use async_trait::async_trait;
    use bytes::Bytes;
    use tokio::sync::{Mutex, mpsc};
    static ROOT_TEST_LOCK: Mutex<()> = Mutex::const_new(());
    fn fixture_addr(guild: u8, key: u8) -> Ipv6Addr {
        app_ula(&[guild; 32], &[key; 32])
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
    async fn admitted_route_appears_and_leaves_with_the_pair() {
        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        let _serial = ROOT_TEST_LOCK.lock().await;
        let local = [0x11u8; 32];
        let foreign = [0xfd, 0x8f, 0x78, 0xa9, 0x2c, 0xaa];
        let (conn, _other) = ends();
        let mut opened = open_app_tun(
            local,
            [0xab; 32],
            [0x22; 32],
            [0x22; 32],
            fixture_addr(0x11, 0xcd),
            conn,
        )
        .await;
        assert!(opened.is_up());
        opened.sync_admitted_routes(&[foreign]).unwrap();
        let shown = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&shown.stdout);
        assert!(text.contains("fd8f:78a9:2caa::/48"), "{text}");
        assert!(!text.contains("fd81:2b22:8aca::/48"));
        assert!(!text.contains("fd12:3456:789a::/48"));
        unsafe {
            std::env::set_var("MJ_APP_ROUTE_FAIL", "1");
        }
        assert!(opened.sync_admitted_routes(&[]).is_err());
        let stuck = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&stuck.stdout).contains("fd8f:78a9:2caa::/48"));
        unsafe {
            std::env::remove_var("MJ_APP_ROUTE_FAIL");
        }
        opened.sync_admitted_routes(&[]).unwrap();
        let shown = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&shown.stdout);
        assert!(!text.contains("fd8f:78a9:2caa::/48"), "{text}");

        unsafe {
            std::env::set_var("MJ_APP_ROUTE_OBSERVE_FAIL", "1");
        }
        assert!(opened.sync_admitted_routes(&[foreign]).is_err());
        let hidden = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&hidden.stdout).contains("fd8f:78a9:2caa::/48"));
        unsafe {
            std::env::remove_var("MJ_APP_ROUTE_OBSERVE_FAIL");
        }
        opened.sync_admitted_routes(&[]).unwrap();
        let shown = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&shown.stdout).contains("fd8f:78a9:2caa::/48"));

        opened.sync_admitted_routes(&[foreign]).unwrap();
        unsafe {
            std::env::set_var("MJ_APP_ROUTE_OBSERVE_FAIL", "1");
        }
        unsafe {
            std::env::set_var("MJ_APP_ROUTE_DEL_SKIP", "1");
        }
        assert!(opened.sync_admitted_routes(&[]).is_err());
        let stuck = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&stuck.stdout).contains("fd8f:78a9:2caa::/48"));
        unsafe {
            std::env::remove_var("MJ_APP_ROUTE_OBSERVE_FAIL");
        }
        unsafe {
            std::env::remove_var("MJ_APP_ROUTE_DEL_SKIP");
        }
        opened.sync_admitted_routes(&[]).unwrap();
        let shown = Command::new("ip")
            .args(["-6", "route", "show"])
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&shown.stdout).contains("fd8f:78a9:2caa::/48"));
        drop(opened);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn route_install_failure_removes_the_address() {
        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        let _serial = ROOT_TEST_LOCK.lock().await;
        let local = fixture_addr(0x11, 0xab);
        let (conn, _other) = ends();
        unsafe {
            std::env::set_var("MJ_APP_ROUTE_FAIL", "1");
        }
        let opened = open_app_tun(
            [0x11; 32],
            [0xab; 32],
            [0x22; 32],
            [0x22; 32],
            fixture_addr(0x11, 0xcd),
            conn,
        )
        .await;
        unsafe {
            std::env::remove_var("MJ_APP_ROUTE_FAIL");
        }
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
        let _serial = ROOT_TEST_LOCK.lock().await;
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
        while opened.inner.as_ref().unwrap().up.load(Ordering::SeqCst)
            && std::time::Instant::now() < deadline
        {
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod lifecycle_tests;
