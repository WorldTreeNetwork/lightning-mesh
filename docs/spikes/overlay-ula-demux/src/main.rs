//! Isolated demux: an fd /128 kernel route selects one already-open peer,
//! and that selection is gone after the route is deleted.
//! No Iroh endpoint is created. Production meshd is not linked.

use std::collections::HashMap;
use std::io::Read;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::fd::AsRawFd;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const IFACE: &str = "spikedmx";
const HOUSE: &str = "fd81:2b22:8aca::2";
const RANDOM: &str = "fd81:2b22:8aca::99";
const STOCK: &str = "fd12:3456:789a::1";
const OPEN_PEER: Ipv4Addr = Ipv4Addr::new(10, 254, 0, 2);
const CLOSED_PEER: Ipv4Addr = Ipv4Addr::new(10, 254, 0, 3);

fn main() {
    if euid() != 0 {
        eprintln!("need root");
        std::process::exit(2);
    }
    let _ = Command::new("ip").args(["link", "del", IFACE]).status();

    let mut cfg = tun::Configuration::default();
    cfg.tun_name(IFACE).mtu(1280).up();
    let dev = tun::create(&cfg).expect("tun");
    set_nonblock(dev.as_raw_fd());
    ip(&["link", "set", IFACE, "up"]).expect("link up");
    ip(&["-6", "addr", "replace", "fd81:2b22:8aca::1/128", "dev", IFACE, "nodad"]).expect("local");

    let table: Arc<Mutex<HashMap<Ipv6Addr, Ipv4Addr>>> = Arc::new(Mutex::new(HashMap::new()));
    let open_hits = Arc::new(Mutex::new(0u64));
    let dropped = Arc::new(Mutex::new(0u64));
    let stop = Arc::new(Mutex::new(false));

    let table_r = Arc::clone(&table);
    let hits_r = Arc::clone(&open_hits);
    let dropped_r = Arc::clone(&dropped);
    let stop_r = Arc::clone(&stop);
    let reader = thread::spawn(move || {
        let mut dev = dev;
        let mut buf = [0u8; 1500];
        while !*stop_r.lock().unwrap() {
            match dev.read(&mut buf) {
                Ok(0) => break,
                Ok(n) if n >= 40 && buf[0] >> 4 == 6 => {
                    let dest = Ipv6Addr::from(<[u8; 16]>::try_from(&buf[24..40]).unwrap());
                    match table_r.lock().unwrap().get(&dest).copied() {
                        Some(peer) if peer == OPEN_PEER => *hits_r.lock().unwrap() += 1,
                        _ => *dropped_r.lock().unwrap() += 1,
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });

    let via = overlay_link_local(OPEN_PEER).to_string();
    let via_closed = overlay_link_local(CLOSED_PEER).to_string();
    let _ = ip(&["-6", "neigh", "replace", &via, "dev", IFACE, "nud", "permanent"]);
    let _ = ip(&["-6", "neigh", "replace", &via_closed, "dev", IFACE, "nud", "permanent"]);
    let house_route = ip(&["-6", "route", "replace", &format!("{HOUSE}/128"), "via", &via, "dev", IFACE, "onlink"]);
    refresh(&table);
    println!("table_after_house={}", table.lock().unwrap().len());
    thread::sleep(Duration::from_millis(150));
    let ping_house = ping(HOUSE);
    thread::sleep(Duration::from_millis(250));
    let during_hits = *open_hits.lock().unwrap();
    let during_dropped = *dropped.lock().unwrap();

    let _ = ip(&["-6", "route", "del", &format!("{HOUSE}/128"), "via", &via, "dev", IFACE]);
    refresh(&table);
    let house: Ipv6Addr = HOUSE.parse().unwrap();
    let still_mapped = table.lock().unwrap().contains_key(&house);
    let replay = table.lock().unwrap().get(&house).copied();
    let ping_after = ping(HOUSE);
    thread::sleep(Duration::from_millis(200));
    let after_hits = *open_hits.lock().unwrap();

    let _ = ip(&["-6", "route", "replace", &format!("{RANDOM}/128"), "via", &via_closed, "dev", IFACE, "onlink"]);
    refresh(&table);
    let ping_random = ping(RANDOM);
    thread::sleep(Duration::from_millis(250));
    let random: Ipv6Addr = RANDOM.parse().unwrap();
    let random_mapped = table.lock().unwrap().get(&random).copied();
    let hits_before_random_count = after_hits;
    let hits_after_random = *open_hits.lock().unwrap();
    let _ = ip(&["-6", "route", "del", &format!("{RANDOM}/128"), "via", &via_closed, "dev", IFACE]);
    refresh(&table);

    let ping_stock = ping(STOCK);
    let stock: Ipv6Addr = STOCK.parse().unwrap();
    let stock_mapped = table.lock().unwrap().contains_key(&stock);
    let udp6 = std::fs::read_to_string("/proc/self/net/udp6").unwrap_or_default();
    let bound_fd = udp6.to_ascii_lowercase().contains("fd812b22");

    *stop.lock().unwrap() = true;
    let _ = reader.join();
    let _ = Command::new("ip").args(["link", "del", IFACE]).status();

    println!("house_route_ok={}", house_route.is_ok());
    println!("via_open={via}");
    println!("ping_house_ok={ping_house}");
    println!("during_open={during_hits} during_dropped={during_dropped}");
    println!("after_delete_still_mapped={still_mapped}");
    println!("replay_after_delete={}", replay.map(|a| a.to_string()).unwrap_or_else(|| "none".into()));
    println!("ping_after_delete_ok={ping_after}");
    println!("after_open={after_hits}");
    println!("ping_random_ok={ping_random}");
    println!("random_mapped={}", random_mapped.map(|a| a.to_string()).unwrap_or_else(|| "none".into()));
    println!("open_hits_unchanged_for_random={}", hits_after_random == hits_before_random_count);
    println!("ping_stock_ok={ping_stock}");
    println!("stock_mapped={stock_mapped}");
    println!("fd_bound_on_udp6={bound_fd}");
}

fn overlay_link_local(addr: Ipv4Addr) -> Ipv6Addr {
    let o = addr.octets();
    let host = u16::from_be_bytes([o[2], o[3]]);
    Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, host)
}

fn refresh(table: &Mutex<HashMap<Ipv6Addr, Ipv4Addr>>) {
    let out = Command::new("ip").args(["-6", "route", "show", "dev", IFACE]).output().expect("route show");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut next = HashMap::new();
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        let Some(prefix) = toks.first() else { continue };
        if prefix.starts_with("fe80:") || *prefix == "any" || *prefix == "default" {
            continue;
        }
        let Some((dest, 128)) = parse_host(prefix) else { continue };
        let via = toks.iter().position(|t| *t == "via").and_then(|i| toks.get(i + 1));
        let Some(via) = via.and_then(|s| s.parse::<Ipv6Addr>().ok()) else { continue };
        let Some(peer) = fe80_to_overlay(via) else { continue };
        next.insert(dest, peer);
    }
    *table.lock().unwrap() = next;
}

fn parse_host(prefix: &str) -> Option<(Ipv6Addr, u8)> {
    match prefix.split_once('/') {
        Some((ip, len)) => Some((ip.parse().ok()?, len.parse().ok()?)),
        None => Some((prefix.parse().ok()?, 128)),
    }
}

fn fe80_to_overlay(addr: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = addr.segments();
    if s[0] != 0xfe80 || s[1..7].iter().any(|x| *x != 0) {
        return None;
    }
    let host = s[7];
    Some(Ipv4Addr::new(10, 254, (host >> 8) as u8, (host & 0xff) as u8))
}

fn ping(dest: &str) -> bool {
    Command::new("ping")
        .args(["-6", "-c", "1", "-W", "1", dest])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn ip(args: &[&str]) -> Result<(), ()> {
    let ok = Command::new("ip").args(args).status().map(|s| s.success()).unwrap_or(false);
    if ok { Ok(()) } else { Err(()) }
}

fn euid() -> u32 {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    text.lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|rest| rest.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn set_nonblock(fd: i32) {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        panic!("fcntl O_NONBLOCK failed");
    }
}
