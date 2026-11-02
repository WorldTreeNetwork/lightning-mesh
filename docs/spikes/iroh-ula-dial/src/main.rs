//! Bind two iroh 1.0.0 endpoints the way mjolnir-meshd binds one backhaul
//! address, each on a house-shaped /128, and dial one unidirectional payload
//! across a veth. Relays stay off. This does not change production code.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::process::Command;

use anyhow::{anyhow, Context};
use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointAddr, RelayMode, SecretKey};

const ALPN: &[u8] = b"mjolnir-ula-spike/1";
/// Fixture router Display for guild 0x11 × 32 and node id `node-example`.
const ADDR_A: &str = "fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df";
/// A second /128 in the same /48. The dial only needs a distinct house address.
const ADDR_B: &str = "fd81:2b22:8aca:0:1111:2222:3333:4444";
const PORT_A: u16 = 40123;
const PORT_B: u16 = 40124;
const IF_A: &str = "ula-spike-a";
const IF_B: &str = "ula-spike-b";

struct Veth;

impl Drop for Veth {
    fn drop(&mut self) {
        let _ = Command::new("sudo")
            .args(["-n", "ip", "link", "del", IF_A])
            .status();
    }
}

fn sudo(args: &[&str]) -> anyhow::Result<()> {
    let out = Command::new("sudo")
        .arg("-n")
        .args(args)
        .output()
        .with_context(|| format!("spawn sudo {}", args.join(" ")))?;
    if out.status.success() {
        return Ok(());
    }
    anyhow::bail!(
        "sudo {} failed ({}): {}",
        args.join(" "),
        out.status,
        String::from_utf8_lossy(&out.stderr).trim()
    );
}

fn setup() -> anyhow::Result<Veth> {
    let _ = Command::new("sudo")
        .args(["-n", "ip", "link", "del", IF_A])
        .status();
    sudo(&["ip", "link", "add", IF_A, "type", "veth", "peer", "name", IF_B])?;
    let guard = Veth;
    sudo(&["ip", "link", "set", IF_A, "up"])?;
    sudo(&["ip", "link", "set", IF_B, "up"])?;
    sudo(&["ip", "-6", "addr", "add", &format!("{ADDR_A}/128"), "dev", IF_A, "nodad"])?;
    sudo(&["ip", "-6", "addr", "add", &format!("{ADDR_B}/128"), "dev", IF_B, "nodad"])?;
    sudo(&["ip", "-6", "route", "replace", &format!("{ADDR_B}/128"), "dev", IF_A])?;
    sudo(&["ip", "-6", "route", "replace", &format!("{ADDR_A}/128"), "dev", IF_B])?;
    Ok(guard)
}

fn ping() -> bool {
    Command::new("ping")
        .args(["-6", "-c", "1", "-W", "2", "-I", ADDR_B, ADDR_A])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn report(ep: &Endpoint, label: &str, ip: Ipv6Addr) -> (usize, usize) {
    let bound = ep.bound_sockets();
    let bound_hits = bound.iter().filter(|s| s.ip() == IpAddr::V6(ip)).count();
    let advertised: Vec<_> = ep.addr().ip_addrs().cloned().collect();
    let advertised_hits = advertised
        .iter()
        .filter(|s| s.ip() == IpAddr::V6(ip))
        .count();
    println!(
        "{label} bound_sockets={bound:?} bound_ula_count={bound_hits} advertised={advertised:?} advertised_ula_count={advertised_hits}"
    );
    (bound_hits, advertised_hits)
}

async fn bind_endpoint(secret: [u8; 32], addr: SocketAddr) -> anyhow::Result<Endpoint> {
    Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Disabled)
        .secret_key(SecretKey::from_bytes(&secret))
        .alpns(vec![ALPN.to_vec()])
        .bind_addr(addr)
        .map_err(|e| anyhow!("bind_addr {addr}: {e:#}"))?
        .bind()
        .await
        .map_err(|e| anyhow!("bind {addr}: {e:#}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr_a: Ipv6Addr = ADDR_A.parse().expect("addr a");
    let addr_b: Ipv6Addr = ADDR_B.parse().expect("addr b");
    let sock_a = SocketAddr::from((addr_a, PORT_A));
    let sock_b = SocketAddr::from((addr_b, PORT_B));

    let _veth = setup().context("veth setup")?;
    let ping_ok = ping();
    println!("ping_ok={ping_ok}");
    if !ping_ok {
        anyhow::bail!("icmp to the fixture /128 failed; iroh was not dialed");
    }

    let ep_a = bind_endpoint([0x11; 32], sock_a).await.context("endpoint A")?;
    let ep_b = bind_endpoint([0x22; 32], sock_b).await.context("endpoint B")?;
    let (bound_a, adv_a) = report(&ep_a, "A", addr_a);
    let (bound_b, adv_b) = report(&ep_b, "B", addr_b);
    let dial_target = EndpointAddr::new(ep_a.id()).with_ip_addr(sock_a);
    println!("dial_target={dial_target:?}");

    let accept_ep = ep_a.clone();
    let accept = tokio::spawn(async move {
        let incoming = accept_ep.accept().await.context("accept returned none")?;
        let conn = incoming
            .await
            .map_err(|e| anyhow!("accept handshake: {e:#}"))?;
        let remote = format!("{}", conn.remote_id());
        let mut recv = conn
            .accept_uni()
            .await
            .map_err(|e| anyhow!("accept_uni: {e:#}"))?;
        let msg = recv
            .read_to_end(16)
            .await
            .map_err(|e| anyhow!("read: {e:#}"))?;
        Ok::<_, anyhow::Error>((msg, remote))
    });

    let conn = match ep_b.connect(dial_target, ALPN).await {
        Ok(conn) => conn,
        Err(e) => {
            let text = format!("{e:#}");
            println!("duplicate_error={}", mentions_duplicate(&text));
            println!("connect_error={text}");
            anyhow::bail!("connect failed");
        }
    };
    println!("dial_remote_id={}", conn.remote_id());
    let mut send = conn
        .open_uni()
        .await
        .map_err(|e| anyhow!("open_uni: {e:#}"))?;
    send.write_all(b"ula")
        .await
        .map_err(|e| anyhow!("write: {e:#}"))?;
    send.finish().map_err(|e| anyhow!("finish: {e:#}"))?;

    let (msg, accept_remote) = accept.await.context("accept task")??;
    println!("payload={msg:?} accept_remote={accept_remote}");
    let roundtrip = msg == b"ula";
    let duplicate_error = false;
    println!(
        "RESULT ping_ok={ping_ok} roundtrip={roundtrip} duplicate_error={duplicate_error} bound_a={bound_a} bound_b={bound_b} advertised_a={adv_a} advertised_b={adv_b}"
    );
    if !roundtrip || bound_a != 1 || bound_b != 1 || adv_a > 1 || adv_b > 1 {
        anyhow::bail!("assertion failed");
    }
    ep_a.close().await;
    ep_b.close().await;
    Ok(())
}

fn mentions_duplicate(text: &str) -> bool {
    text.to_ascii_lowercase().contains("duplicate")
}
