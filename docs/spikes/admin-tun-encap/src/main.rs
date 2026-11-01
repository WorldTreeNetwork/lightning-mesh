//! Two real TUN devices, each in its own netns, pass one ICMP echo through
//! `spawn_encap_pair`. Addresses come from `app_ula`. The peer conn is the
//! one labeled with the 32-byte node id. A second label stays empty.
//! This does not bind the ULA as an Iroh candidate.

use std::net::Ipv6Addr;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use mjolnir_mesh::tun::{app_ula, spawn_encap_pair, DatagramConn, EncapError};
use tokio::sync::{mpsc, Mutex};

const NODE_B: [u8; 32] = [0x22; 32];
const OTHER: [u8; 32] = [0x33; 32];

#[derive(Clone)]
struct LabeledConn {
    peer: [u8; 32],
    tx: mpsc::Sender<Bytes>,
    rx: Arc<Mutex<mpsc::Receiver<Bytes>>>,
    sent: Arc<Mutex<Vec<Bytes>>>,
}

#[async_trait]
impl DatagramConn for LabeledConn {
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

fn pair(peer_of_a: [u8; 32], peer_of_b: [u8; 32]) -> (LabeledConn, LabeledConn) {
    let (a_tx, b_rx) = mpsc::channel(8);
    let (b_tx, a_rx) = mpsc::channel(8);
    let sent_a = Arc::new(Mutex::new(Vec::new()));
    let sent_b = Arc::new(Mutex::new(Vec::new()));
    (
        LabeledConn {
            peer: peer_of_a,
            tx: a_tx,
            rx: Arc::new(Mutex::new(a_rx)),
            sent: sent_a,
        },
        LabeledConn {
            peer: peer_of_b,
            tx: b_tx,
            rx: Arc::new(Mutex::new(b_rx)),
            sent: sent_b,
        },
    )
}

fn ip(args: &[&str]) {
    let status = Command::new("ip").args(args).status().expect("ip");
    assert!(status.success(), "ip {args:?} failed");
}

fn ping(ns: &str, dest: &str) -> bool {
    Command::new("ip")
        .args(["netns", "exec", ns, "ping", "-6", "-c", "1", "-W", "2", dest])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn tun(name: &str) -> tun::AsyncDevice {
    let mut config = tun::Configuration::default();
    config.tun_name(name).mtu(1280).up();
    tun::create_as_async(&config).expect("create tun")
}

fn icmp(pkt: &[u8], dest: Ipv6Addr, kind: u8) -> bool {
    pkt.first().is_some_and(|v| v >> 4 == 6)
        && pkt.get(6) == Some(&58)
        && pkt.get(24..40) == Some(dest.octets().as_slice())
        && pkt.get(40) == Some(&kind)
}

struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        for ns in ["spike-ns-a", "spike-ns-b"] {
            let _ = Command::new("ip").args(["netns", "del", ns]).status();
        }
        for dev in ["spikeA", "spikeB"] {
            let _ = Command::new("ip").args(["link", "del", dev]).status();
        }
    }
}

#[tokio::main]
async fn main() {
    let _cleanup = Cleanup;
    let guild = [0x11u8; 32];
    let addr_a = app_ula(&guild, &[0xab; 32]);
    let addr_b = app_ula(&guild, &[0xcd; 32]);
    let a = addr_a.to_string();
    let b = addr_b.to_string();
    println!("addr_a={a}");
    println!("addr_b={b}");
    let fixture_a_ok = a == "fd81:2b22:8aca:1:bd5e:fa7a:5460:697";
    println!("fixture_a_ok={fixture_a_ok}");

    let dev_a = tun("spikeA");
    let dev_b = tun("spikeB");
    ip(&["netns", "add", "spike-ns-a"]);
    ip(&["netns", "add", "spike-ns-b"]);
    ip(&["link", "set", "spikeA", "netns", "spike-ns-a"]);
    ip(&["link", "set", "spikeB", "netns", "spike-ns-b"]);
    for (ns, dev, local, remote) in [
        ("spike-ns-a", "spikeA", a.as_str(), b.as_str()),
        ("spike-ns-b", "spikeB", b.as_str(), a.as_str()),
    ] {
        ip(&["netns", "exec", ns, "ip", "link", "set", dev, "up"]);
        ip(&[
            "netns", "exec", ns, "ip", "-6", "addr", "add", &format!("{local}/128"), "dev", dev, "nodad",
        ]);
        ip(&[
            "netns", "exec", ns, "ip", "-6", "route", "add", &format!("{remote}/128"), "dev", dev,
        ]);
    }

    let seam_off = ping("spike-ns-a", &b);
    println!("seam_off_ping_ok={seam_off}");

    let (conn_a, conn_b) = pair(NODE_B, [0x11; 32]);
    let other_sent = Arc::new(Mutex::new(Vec::new()));
    let _other = LabeledConn {
        peer: OTHER,
        tx: conn_a.tx.clone(),
        rx: conn_a.rx.clone(),
        sent: other_sent.clone(),
    };
    assert_eq!(conn_a.peer, NODE_B);
    assert_ne!(_other.peer, NODE_B);

    let (write_a, read_a) = dev_a.split().expect("split a");
    let (write_b, read_b) = dev_b.split().expect("split b");
    let handles_a = spawn_encap_pair(read_a, write_a, conn_a.clone(), 1280);
    let handles_b = spawn_encap_pair(read_b, write_b, conn_b.clone(), 1280);

    let seam_on = ping("spike-ns-a", &b);
    println!("seam_on_ping_ok={seam_on}");
    let sent_a = conn_a.sent.lock().await.clone();
    let sent_b = conn_b.sent.lock().await.clone();
    let req = sent_a.iter().filter(|p| icmp(p, addr_b, 128)).count();
    let rep = sent_b.iter().filter(|p| icmp(p, addr_a, 129)).count();
    let other = other_sent.lock().await.len();
    println!("echo_request_on_peer={req}");
    println!("echo_reply_on_peer={rep}");
    println!("other_peer_packets={other}");

    handles_a.abort();
    handles_b.abort();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let after_abort = ping("spike-ns-a", &b);
    println!("seam_abort_ping_ok={after_abort}");

    let ok = fixture_a_ok && !seam_off && seam_on && req >= 1 && rep >= 1 && other == 0 && !after_abort;
    if !ok {
        std::process::exit(1);
    }
}
