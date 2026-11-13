use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};

use crate::crdt::hlc::HLC;
use crate::crdt::merge::MergeResult;

/// A DHCP lease entry in the mesh CRDT.
///
/// Keyed by MAC address at `/devices/{mac}`. One entry per device.
/// Uses `IpAddr` (v4/v6 enum) for forward compatibility with IPv6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseEntry {
    pub mac: [u8; 6],
    /// IPv4 today; IPv6 forward-compatible without schema change.
    pub ip: IpAddr,
    pub hostname: Option<String>,
    pub router_id: String,
    /// Unix timestamp in seconds; daemon reaps expired entries.
    pub expiry: u64,
    pub hlc: HLC,
}

/// MAC → lease. Key is [`mac_key`].
pub type LeaseBook = BTreeMap<String, LeaseEntry>;

/// Lowercase `aa:bb:cc:dd:ee:ff`.
pub fn mac_key(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

/// Parse `aa:bb:cc:dd:ee:ff` or `aa-bb-cc-dd-ee-ff`.
pub fn parse_mac(s: &str) -> Option<[u8; 6]> {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 12 {
        return None;
    }
    let mut mac = [0u8; 6];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        mac[i] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(mac)
}

/// `10.42.x.y` → `10.42.x.1`. Other v4 is None (do not invent a gw).
pub fn gateway_v4(ip: Ipv4Addr) -> Option<Ipv4Addr> {
    let o = ip.octets();
    if o[0] == 10 && o[1] == 42 {
        Some(Ipv4Addr::new(10, 42, o[2], 1))
    } else {
        None
    }
}

/// Merge one MAC's lease. Last-writer-wins on HLC: the latest ACK is the
/// address other APs must OFFER (keep-current, not first-ever).
pub fn merge_lease(local: Option<&LeaseEntry>, incoming: &LeaseEntry) -> MergeResult<LeaseEntry> {
    match local {
        None => MergeResult::Inserted,
        Some(existing) => match incoming.hlc.cmp(&existing.hlc) {
            std::cmp::Ordering::Greater => MergeResult::Updated,
            _ => MergeResult::Unchanged,
        },
    }
}

/// Apply `incoming` into `book`. Conflict winner is stored (first IP kept).
pub fn apply_lease(book: &mut LeaseBook, incoming: LeaseEntry) -> MergeResult<LeaseEntry> {
    let key = mac_key(&incoming.mac);
    let result = merge_lease(book.get(&key), &incoming);
    match &result {
        MergeResult::Inserted | MergeResult::Updated => {
            book.insert(key, incoming);
        }
        MergeResult::Unchanged | MergeResult::Conflict { .. } => {}
    }
    result
}

/// The guest-address space the roam config covers: one `/24` per node.
const ROAM_SPACE: [u8; 2] = [10, 42];

/// Is `ip` a mesh guest address: inside `10.42.<1..=254>.0/24`.
fn is_mesh_guest(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == ROAM_SPACE[0] && o[1] == ROAM_SPACE[1] && (1..=254).contains(&o[2])
}

/// dnsmasq `dhcp-hostsfile` lines, `mac,ip`, sorted: one per lease in the
/// mesh guest space. A lease another node vended is confirmed here through the
/// static roam config. A lease in this node's own `/24` is listed too: a phone
/// that keeps renewing on another node outlives its lease here, and the
/// reservation stops this node handing its address to someone else. dnsmasq
/// re-reads this file on SIGHUP.
pub fn render_hostsfile(book: &LeaseBook) -> String {
    let mut lines: Vec<String> = book
        .values()
        .filter_map(|e| {
            let IpAddr::V4(ip) = e.ip else {
                return None;
            };
            is_mesh_guest(ip).then(|| format!("{},{}", mac_key(&e.mac), ip))
        })
        .collect();
    lines.sort();
    lines.push(String::new());
    lines.join("\n")
}

/// Static dnsmasq config that lets this node confirm an address in any other
/// node's `/24`: a `shared-network` plus a `static` range per foreign subnet,
/// each telling the client its *home* gateway and DNS. dnsmasq reads it only
/// at start, so a change here needs a restart; the reservations themselves
/// live in `hosts_path` ([`render_hostsfile`]) and need only SIGHUP.
pub fn render_roam_static_conf(
    client_iface: &str,
    own_subnet: &Ipv4Net,
    hosts_path: &Path,
) -> String {
    let mut out = format!("dhcp-hostsfile={}\n", hosts_path.display());
    for x in 1..=254u8 {
        let net = Ipv4Addr::new(ROAM_SPACE[0], ROAM_SPACE[1], x, 0);
        if own_subnet.contains(&net) {
            continue;
        }
        let gw = Ipv4Addr::new(ROAM_SPACE[0], ROAM_SPACE[1], x, 1);
        out.push_str(&format!(
            "shared-network={client_iface},{net}\n\
             dhcp-range=set:g{x},{net},static,255.255.255.0,12h\n\
             dhcp-option=tag:g{x},option:router,{gw}\n\
             dhcp-option=tag:g{x},option:dns-server,{gw}\n"
        ));
    }
    out
}

/// One IPv4 line of dnsmasq's lease file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseFileEntry {
    pub mac: [u8; 6],
    pub ip: Ipv4Addr,
    pub hostname: Option<String>,
    /// Unix seconds. dnsmasq's `0` (infinite) is carried as `u64::MAX`.
    pub expiry: u64,
}

/// What one poll of the lease file saw: [`mac_key`] → (address, expiry).
pub type LeasePoll = BTreeMap<String, (Ipv4Addr, u64)>;

/// Parse dnsmasq's lease file:
/// `<expiry-unix> <mac> <ip> <hostname|*> <client-id|*>` per line.
///
/// IPv6 and `duid` lines are skipped, as is anything outside `10.42.0.0/16`.
/// dnsmasq rewrites the file in place, so a read can catch it half-written:
/// a line with fewer than five fields, or a last line with no newline, is
/// dropped rather than trusted with a cut-off address.
pub fn parse_lease_file(text: &str) -> Vec<LeaseFileEntry> {
    let complete = match text.rfind('\n') {
        Some(end) => &text[..end],
        None => return Vec::new(),
    };
    complete
        .lines()
        .filter_map(|line| {
            let mut f = line.split_whitespace();
            let expiry: u64 = f.next()?.parse().ok()?;
            let mac_s = f.next()?;
            let ip: Ipv4Addr = f.next()?.parse().ok()?;
            let hostname = f.next()?;
            f.next()?;
            // A v4 MAC is exactly six colon-separated octets; an IAID or a
            // DUID in this column is not.
            if mac_s.len() != 17 {
                return None;
            }
            let mac = parse_mac(mac_s)?;
            let o = ip.octets();
            if o[0] != ROAM_SPACE[0] || o[1] != ROAM_SPACE[1] {
                return None;
            }
            Some(LeaseFileEntry {
                mac,
                ip,
                hostname: (hostname != "*").then(|| hostname.to_string()),
                expiry: if expiry == 0 { u64::MAX } else { expiry },
            })
        })
        .collect()
}

/// The lease-file entries that are a fresh ACK since `prev` and so should be
/// stamped and published.
///
/// An entry is fresh when its MAC was not in `prev`, or its address changed,
/// or its expiry advanced (a renewal). On the first poll after start `prev` is
/// empty and every line looks new, so the book decides instead: a line is
/// published only if the book lacks the MAC or the line expires later than the
/// book's entry. Lease times are equal fleet-wide, so a later expiry is a later
/// ACK. That keeps a stale row here from overriding a newer ACK on another
/// node, and still picks up a renewal dnsmasq granted while meshd was down.
pub fn fresh_acks(
    prev: &LeasePoll,
    current: &[LeaseFileEntry],
    book: &LeaseBook,
    first_poll: bool,
) -> Vec<LeaseFileEntry> {
    current
        .iter()
        .filter(|e| {
            let key = mac_key(&e.mac);
            if first_poll {
                return book.get(&key).is_none_or(|held| e.expiry > held.expiry);
            }
            match prev.get(&key) {
                None => true,
                Some((ip, expiry)) => *ip != e.ip || e.expiry > *expiry,
            }
        })
        .cloned()
        .collect()
}

/// Drop leases whose `expiry` is at or before `now_unix`. Returns the removed
/// keys so the caller can gossip `LeaseRelease`.
pub fn reap_expired(book: &mut LeaseBook, now_unix: u64) -> Vec<LeaseEntry> {
    let mut gone = Vec::new();
    book.retain(|_, e| {
        if e.expiry <= now_unix {
            gone.push(e.clone());
            false
        } else {
            true
        }
    });
    gone
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;

    use super::*;
    use crate::crdt::merge::MergeResult;

    fn hlc(wall: u64, node: &str) -> HLC {
        HLC {
            wall_clock: wall,
            counter: 0,
            node_id: node.to_string(),
        }
    }

    fn lease(mac: [u8; 6], ip: &str, router: &str, wall: u64) -> LeaseEntry {
        LeaseEntry {
            mac,
            ip: IpAddr::V4(ip.parse().unwrap()),
            hostname: None,
            router_id: router.to_string(),
            expiry: 1_700_000_000,
            hlc: hlc(wall, router),
        }
    }

    #[test]
    fn postcard_roundtrip() {
        let original = LeaseEntry {
            mac: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
            hostname: Some("laptop".to_string()),
            router_id: "router-a".to_string(),
            expiry: 1_700_000_000,
            hlc: HLC {
                wall_clock: 1_700_000_000_000,
                counter: 0,
                node_id: "router-a".to_string(),
            },
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: LeaseEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn parse_mac_colon_or_dash() {
        let want = [0xde, 0x45, 0x53, 0xf5, 0xf7, 0x74];
        assert_eq!(parse_mac("de:45:53:f5:f7:74"), Some(want));
        assert_eq!(parse_mac("DE-45-53-F5-F7-74"), Some(want));
        assert_eq!(mac_key(&want), "de:45:53:f5:f7:74");
    }

    #[test]
    fn gateway_is_slash24_dot1_in_mesh_space() {
        assert_eq!(
            gateway_v4("10.42.203.146".parse().unwrap()),
            Some("10.42.203.1".parse().unwrap())
        );
        assert_eq!(gateway_v4("192.168.1.42".parse().unwrap()), None);
    }

    #[test]
    fn latest_ack_wins_even_when_the_ip_changed() {
        let mac = [0xde, 0x45, 0x53, 0xf5, 0xf7, 0x74];
        let home = lease(mac, "10.42.242.243", "wr3000s-a", 100);
        let current = lease(mac, "10.42.203.246", "indoor-c", 200);
        let mut book = LeaseBook::new();
        assert!(matches!(apply_lease(&mut book, home), MergeResult::Inserted));
        assert!(matches!(
            apply_lease(&mut book, current),
            MergeResult::Updated
        ));
        assert_eq!(
            book[&mac_key(&mac)].ip,
            "10.42.203.246".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn stale_ack_does_not_roll_the_ip_back() {
        let mac = [0xde, 0x45, 0x53, 0xf5, 0xf7, 0x74];
        let mut book = LeaseBook::new();
        apply_lease(&mut book, lease(mac, "10.42.203.246", "indoor-c", 200));
        apply_lease(&mut book, lease(mac, "10.42.242.243", "wr3000s-a", 100));
        assert_eq!(
            book[&mac_key(&mac)].ip,
            "10.42.203.246".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn reap_expired_drops_only_old_rows() {
        let mut book = LeaseBook::new();
        let mut live = lease([1u8; 6], "10.42.1.10", "a", 1);
        live.expiry = 200;
        let mut dead = lease([2u8; 6], "10.42.1.11", "a", 1);
        dead.expiry = 50;
        apply_lease(&mut book, live);
        apply_lease(&mut book, dead);
        let gone = reap_expired(&mut book, 100);
        assert_eq!(gone.len(), 1);
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn same_ip_updates_router_on_newer_hlc() {
        let mac = [1u8; 6];
        let a = lease(mac, "10.42.203.146", "indoor-c", 100);
        let b = lease(mac, "10.42.203.146", "wr3000s-a", 200);
        let mut book = LeaseBook::new();
        apply_lease(&mut book, a);
        assert!(matches!(
            apply_lease(&mut book, b),
            MergeResult::Updated
        ));
        assert_eq!(book[&mac_key(&mac)].router_id, "wr3000s-a");
    }

    fn net(s: &str) -> Ipv4Net {
        s.parse().unwrap()
    }

    #[test]
    fn first_poll_keeps_a_newer_ack_from_another_node() {
        // Our file still holds m at .61.106 until 1000; node b ACKed m at
        // .242.50 later (expiry 2000). A restart must not bring .61.106 back.
        let mac = [0xaa, 0, 0, 0, 0, 1];
        let mut book = LeaseBook::new();
        let mut held = lease(mac, "10.42.242.50", "b", 5);
        held.expiry = 2000;
        apply_lease(&mut book, held);
        let file = [LeaseFileEntry {
            mac,
            ip: "10.42.61.106".parse().unwrap(),
            hostname: None,
            expiry: 1000,
        }];
        assert!(fresh_acks(&LeasePoll::new(), &file, &book, true).is_empty());
    }

    #[test]
    fn first_poll_publishes_a_renewal_granted_while_down() {
        let mac = [0xaa, 0, 0, 0, 0, 1];
        let mut book = LeaseBook::new();
        let mut held = lease(mac, "10.42.61.106", "a", 5);
        held.expiry = 1000;
        apply_lease(&mut book, held);
        let file = [LeaseFileEntry {
            mac,
            ip: "10.42.61.106".parse().unwrap(),
            hostname: None,
            expiry: 1000 + 43_200,
        }];
        assert_eq!(fresh_acks(&LeasePoll::new(), &file, &book, true).len(), 1);
    }

    #[test]
    fn hostsfile_lists_every_mesh_guest_lease() {
        let mut book = LeaseBook::new();
        apply_lease(&mut book, lease([0xde, 0x45, 0x53, 0xf5, 0xf7, 0x74], "10.42.203.146", "c", 1));
        apply_lease(&mut book, lease([0xaa, 0, 0, 0, 0, 1], "10.42.61.106", "a", 1));
        // Off-mesh v4 and v6 are left out.
        apply_lease(&mut book, lease([0xaa, 0, 0, 0, 0, 2], "10.42.242.100", "me", 1));
        apply_lease(&mut book, lease([0xaa, 0, 0, 0, 0, 3], "192.168.1.42", "x", 1));
        let mut v6 = lease([0xaa, 0, 0, 0, 0, 4], "10.42.9.9", "x", 1);
        v6.ip = "fd00::1".parse().unwrap();
        apply_lease(&mut book, v6);
        assert_eq!(
            render_hostsfile(&book),
            "aa:00:00:00:00:01,10.42.61.106\naa:00:00:00:00:02,10.42.242.100\nde:45:53:f5:f7:74,10.42.203.146\n"
        );
        assert_eq!(render_hostsfile(&LeaseBook::new()), "");
    }

    #[test]
    fn static_conf_covers_every_foreign_subnet_and_not_our_own() {
        let hosts = Path::new("/tmp/dnsmasq.cfg01411c.d/.mjolnir-roam.hosts");
        let conf = render_roam_static_conf("br-lan", &net("10.42.242.0/24"), hosts);
        let lines: Vec<&str> = conf.lines().collect();
        assert_eq!(
            &lines[..5],
            &[
                "dhcp-hostsfile=/tmp/dnsmasq.cfg01411c.d/.mjolnir-roam.hosts",
                "shared-network=br-lan,10.42.1.0",
                "dhcp-range=set:g1,10.42.1.0,static,255.255.255.0,12h",
                "dhcp-option=tag:g1,option:router,10.42.1.1",
                "dhcp-option=tag:g1,option:dns-server,10.42.1.1",
            ]
        );
        assert_eq!(lines.len(), 1 + 253 * 4);
        assert!(conf.ends_with("dhcp-option=tag:g254,option:dns-server,10.42.254.1\n"));
        assert!(conf.contains("shared-network=br-lan,10.42.61.0\n"));
        assert!(!conf.contains("10.42.242."));
        assert!(!conf.contains("10.42.0.0") && !conf.contains("10.42.255.0"));
        assert_eq!(
            conf,
            render_roam_static_conf("br-lan", &net("10.42.242.0/24"), hosts),
            "output is deterministic"
        );
    }

    #[test]
    fn lease_file_parser_keeps_mesh_v4_lines_only() {
        let text = "\
1731500000 aa:00:00:00:00:01 10.42.61.106 pixel 01:aa:00:00:00:00:01
0 AA:00:00:00:00:02 10.42.61.107 * *
1731500000 aa:00:00:00:00:03 192.168.1.50 laptop *
duid 00:01:00:01:2c:6a:9b:1e:aa:00:00:00:00:01
1731500000 1234567 fd00::106 host 00:01:00:01:2c:6a:9b:1e:aa:00:00:00:00:04
not a lease line
1731500000 aa:00:00:00:00:05 10.42.61.108 short
";
        let got = parse_lease_file(text);
        assert_eq!(
            got,
            vec![
                LeaseFileEntry {
                    mac: [0xaa, 0, 0, 0, 0, 1],
                    ip: "10.42.61.106".parse().unwrap(),
                    hostname: Some("pixel".to_string()),
                    expiry: 1_731_500_000,
                },
                LeaseFileEntry {
                    mac: [0xaa, 0, 0, 0, 0, 2],
                    ip: "10.42.61.107".parse().unwrap(),
                    hostname: None,
                    expiry: u64::MAX,
                },
            ]
        );
    }

    #[test]
    fn lease_file_parser_drops_a_half_written_last_line() {
        // dnsmasq rewrites in place; "10.42.61.10" here is a cut-off .106.
        let text = "1731500000 aa:00:00:00:00:01 10.42.61.106 * *\n\
                    1731500000 aa:00:00:00:00:02 10.42.61.10";
        assert_eq!(parse_lease_file(text).len(), 1);
        assert!(parse_lease_file("1731500000 aa:00:00:00:00:02 10.42.61.106 * *").is_empty());
    }

    fn file_entry(last: u8, ip: &str, expiry: u64) -> LeaseFileEntry {
        LeaseFileEntry {
            mac: [0xaa, 0, 0, 0, 0, last],
            ip: ip.parse().unwrap(),
            hostname: None,
            expiry,
        }
    }

    fn poll_of(entries: &[LeaseFileEntry]) -> LeasePoll {
        entries
            .iter()
            .map(|e| (mac_key(&e.mac), (e.ip, e.expiry)))
            .collect()
    }

    #[test]
    fn fresh_ack_is_a_new_mac() {
        let seen = file_entry(1, "10.42.61.106", 1000);
        let new = file_entry(2, "10.42.61.107", 1000);
        let prev = poll_of(std::slice::from_ref(&seen));
        let got = fresh_acks(&prev, &[seen, new.clone()], &LeaseBook::new(), false);
        assert_eq!(got, vec![new]);
    }

    #[test]
    fn fresh_ack_is_a_changed_address() {
        let prev = poll_of(&[file_entry(1, "10.42.61.106", 1000)]);
        let moved = file_entry(1, "10.42.61.150", 1000);
        let got = fresh_acks(&prev, std::slice::from_ref(&moved), &LeaseBook::new(), false);
        assert_eq!(got, vec![moved]);
    }

    #[test]
    fn fresh_ack_is_an_advanced_expiry_and_nothing_less() {
        let prev = poll_of(&[file_entry(1, "10.42.61.106", 1000)]);
        let book = LeaseBook::new();
        let renewed = file_entry(1, "10.42.61.106", 1600);
        assert_eq!(
            fresh_acks(&prev, std::slice::from_ref(&renewed), &book, false),
            vec![renewed]
        );
        // Unchanged, or an expiry that went backwards, is not an ACK.
        assert!(fresh_acks(&prev, &[file_entry(1, "10.42.61.106", 1000)], &book, false).is_empty());
        assert!(fresh_acks(&prev, &[file_entry(1, "10.42.61.106", 900)], &book, false).is_empty());
    }

    #[test]
    fn restart_does_not_restamp_a_lease_the_book_already_holds() {
        // Book: node-b wrote this MAC at this address after we first leased it.
        let mut book = LeaseBook::new();
        for (mac, ip) in [(1, "10.42.61.106"), (2, "10.42.203.9")] {
            let mut held = lease([0xaa, 0, 0, 0, 0, mac], ip, "node-b", 500);
            held.expiry = 1000;
            apply_lease(&mut book, held);
        }
        let same = file_entry(1, "10.42.61.106", 1000);
        // Same MAC, an older row here than the book's ACK elsewhere.
        let differs = file_entry(2, "10.42.61.107", 900);
        let unknown = file_entry(3, "10.42.61.108", 1000);
        let current = [same.clone(), differs.clone(), unknown.clone()];
        let first = fresh_acks(&LeasePoll::new(), &current, &book, true);
        assert_eq!(first, vec![unknown]);
        // Past the first poll the same line is an ordinary new MAC...
        assert_eq!(fresh_acks(&LeasePoll::new(), &current, &book, false).len(), 3);
        // ...and a renewal after the restart is published as usual.
        let prev = poll_of(&current);
        let renewed = file_entry(1, "10.42.61.106", 1600);
        assert_eq!(
            fresh_acks(&prev, std::slice::from_ref(&renewed), &book, false),
            vec![renewed]
        );
    }
}
