// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use super::types::{LinkLocalInterface, RawNeighbor};
use super::is_unicast_link_local;

const ALL_NODES: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1);
const COLLECT: Duration = Duration::from_millis(600);
const ECHO_ID: u16 = 0x4c4d; // 'LM'
const ICMP_ECHO_REQUEST: u8 = 128;
const ICMP_ECHO_REPLY: u8 = 129;

pub fn probe_all_nodes(interfaces: &[LinkLocalInterface]) -> (Vec<RawNeighbor>, Option<String>) {
    let targets: Vec<&LinkLocalInterface> = interfaces
        .iter()
        .filter(|i| i.is_up && !i.is_loopback && i.index != 0 && !i.link_local.is_empty())
        .collect();

    if targets.is_empty() {
        return (Vec::new(), None);
    }

    let mut sockets = Vec::new();
    let mut errors = Vec::new();
    for iface in &targets {
        match open_probe_socket(iface.index) {
            Ok(sock) => sockets.push((iface.index, sock)),
            Err(e) => errors.push(format!("{}: {e}", iface.name)),
        }
    }

    if sockets.is_empty() {
        return (
            Vec::new(),
            Some(format!(
                "ICMPv6 probe unavailable ({})",
                errors.join("; ")
            )),
        );
    }

    for (ifindex, sock) in &sockets {
        if let Err(e) = send_echo(sock, *ifindex) {
            tracing::debug!(ifindex, error = %e, "ff02::1 echo send failed");
        }
    }

    let mut found = Vec::new();
    let deadline = Instant::now() + COLLECT;
    let mut buf = [MaybeUninit::<u8>::uninit(); 256];
    while Instant::now() < deadline {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            break;
        }
        for (ifindex, sock) in &sockets {
            let _ = sock.set_read_timeout(Some(Duration::from_millis(20).min(remain)));
            match sock.recv_from(&mut buf) {
                Ok((n, addr)) => {
                    let pkt = unsafe { assume_init_bytes(&buf[..n]) };
                    if n < 8 || pkt[0] != ICMP_ECHO_REPLY {
                        continue;
                    }
                    let Some(v6) = addr.as_socket_ipv6() else {
                        continue;
                    };
                    let ip = *v6.ip();
                    if !is_unicast_link_local(ip) {
                        continue;
                    }
                    found.push(RawNeighbor {
                        ifindex: *ifindex,
                        address: ip,
                        mac: None,
                        state: "reachable".into(),
                        source: "probe",
                    });
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {}
                Err(e) => tracing::debug!(ifindex, error = %e, "icmp recv"),
            }
        }
    }

    let probe_error = if errors.is_empty() {
        None
    } else {
        Some(format!(
            "probe skipped on {}: {}",
            errors.len(),
            errors.join("; ")
        ))
    };
    (found, probe_error)
}

fn open_probe_socket(ifindex: u32) -> io::Result<Socket> {
    let sock = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::ICMPV6))?;
    sock.set_nonblocking(false)?;
    let _ = sock.set_multicast_if_v6(ifindex);
    let _ = sock.set_multicast_hops_v6(1);
    let bind = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, ifindex);
    sock.bind(&SockAddr::from(bind))?;
    Ok(sock)
}

unsafe fn assume_init_bytes(buf: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { &*(buf as *const [MaybeUninit<u8>] as *const [u8]) }
}

fn send_echo(sock: &Socket, ifindex: u32) -> io::Result<usize> {
    let mut pkt = [0u8; 16];
    pkt[0] = ICMP_ECHO_REQUEST;
    pkt[4..6].copy_from_slice(&ECHO_ID.to_be_bytes());
    pkt[6..8].copy_from_slice(&1u16.to_be_bytes());
    pkt[8..].copy_from_slice(b"ltn-mesh");
    let dest = SocketAddrV6::new(ALL_NODES, 0, 0, ifindex);
    sock.send_to(&pkt, &SockAddr::from(dest))
}
