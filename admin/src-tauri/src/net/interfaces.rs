// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv6Addr};

use super::types::LinkLocalInterface;
use super::{is_unicast_link_local, is_unique_local, scoped_addr};

/// `if-addrs` on this host returns GUA/IPv4 but silently drops `fe80::` (and
/// therefore IPv6-only veths). Linux sources of truth: `/sys/class/net` for
/// the iface list, `/proc/net/if_inet6` for every IPv6 address.
pub fn collect_interfaces() -> Result<Vec<LinkLocalInterface>, String> {
    let mut by_name: BTreeMap<String, LinkLocalInterface> = BTreeMap::new();

    collect_sysfs(&mut by_name)?;
    collect_if_inet6(&mut by_name);
    collect_if_addrs_fallback(&mut by_name);

    Ok(by_name.into_values().collect())
}

fn collect_sysfs(by_name: &mut BTreeMap<String, LinkLocalInterface>) -> Result<(), String> {
    let dir = match std::fs::read_dir("/sys/class/net") {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    for entry in dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let index = if_index(&name);
        by_name
            .entry(name.clone())
            .or_insert_with(|| LinkLocalInterface {
                name: name.clone(),
                index,
                mac: read_mac(&name),
                is_up: oper_up(&name),
                is_loopback: is_loopback_iface(&name),
                link_local: Vec::new(),
                unique_local: Vec::new(),
            });
    }
    Ok(())
}

fn collect_if_inet6(by_name: &mut BTreeMap<String, LinkLocalInterface>) {
    let Ok(text) = std::fs::read_to_string("/proc/net/if_inet6") else {
        return;
    };
    for line in text.lines() {
        let Some(parsed) = parse_if_inet6_line(line) else {
            continue;
        };
        if !is_unicast_link_local(parsed.ip) && !is_unique_local(parsed.ip) {
            continue;
        }
        let entry = by_name
            .entry(parsed.name.clone())
            .or_insert_with(|| LinkLocalInterface {
                name: parsed.name.clone(),
                index: parsed.index,
                mac: read_mac(&parsed.name),
                is_up: oper_up(&parsed.name),
                is_loopback: is_loopback_iface(&parsed.name),
                link_local: Vec::new(),
                unique_local: Vec::new(),
            });
        if entry.index == 0 {
            entry.index = parsed.index;
        }
        if is_unicast_link_local(parsed.ip) {
            let scoped = scoped_addr(parsed.ip, &entry.name);
            if !entry.link_local.contains(&scoped) {
                entry.link_local.push(scoped);
            }
        } else if is_unique_local(parsed.ip) {
            let addr = parsed.ip.to_string();
            if !entry.unique_local.contains(&addr) {
                entry.unique_local.push(addr);
            }
        }
    }
}

fn collect_if_addrs_fallback(by_name: &mut BTreeMap<String, LinkLocalInterface>) {
    let Ok(addrs) = if_addrs::get_if_addrs() else {
        return;
    };
    for a in addrs {
        let index = a.index.unwrap_or_else(|| if_index(&a.name));
        let entry = by_name
            .entry(a.name.clone())
            .or_insert_with(|| LinkLocalInterface {
                name: a.name.clone(),
                index,
                mac: read_mac(&a.name),
                is_up: a.is_oper_up(),
                is_loopback: a.is_loopback() || is_loopback_iface(&a.name),
                link_local: Vec::new(),
                unique_local: Vec::new(),
            });
        if let IpAddr::V6(v6) = a.ip() {
            if is_unicast_link_local(v6) {
                let scoped = scoped_addr(v6, &a.name);
                if !entry.link_local.contains(&scoped) {
                    entry.link_local.push(scoped);
                }
            } else if is_unique_local(v6) {
                let addr = v6.to_string();
                if !entry.unique_local.contains(&addr) {
                    entry.unique_local.push(addr);
                }
            }
        }
    }
}

struct IfInet6 {
    name: String,
    index: u32,
    ip: Ipv6Addr,
}

fn parse_if_inet6_line(line: &str) -> Option<IfInet6> {
    let mut parts = line.split_whitespace();
    let hex = parts.next()?;
    if hex.len() != 32 {
        return None;
    }
    let index = u32::from_str_radix(parts.next()?, 16).ok()?;
    let _prefix = parts.next()?;
    let _scope = parts.next()?;
    let _flags = parts.next()?;
    let name = parts.next()?.to_string();
    let mut bytes = [0u8; 16];
    for (i, slot) in bytes.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(IfInet6 {
        name,
        index,
        ip: Ipv6Addr::from(bytes),
    })
}

#[cfg(unix)]
pub fn if_index(name: &str) -> u32 {
    let Ok(c) = std::ffi::CString::new(name) else {
        return 0;
    };
    unsafe { libc::if_nametoindex(c.as_ptr()) }
}

#[cfg(not(unix))]
pub fn if_index(_name: &str) -> u32 {
    0
}

#[cfg(unix)]
pub fn if_name(index: u32) -> Option<String> {
    if index == 0 {
        return None;
    }
    let mut buf = [0u8; libc::IF_NAMESIZE];
    let ptr = unsafe { libc::if_indextoname(index, buf.as_mut_ptr() as *mut libc::c_char) };
    if ptr.is_null() {
        return None;
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
    Some(cstr.to_string_lossy().into_owned())
}

#[cfg(not(unix))]
pub fn if_name(_index: u32) -> Option<String> {
    None
}

fn read_sys(name: &str, file: &str) -> Option<String> {
    std::fs::read_to_string(format!("/sys/class/net/{name}/{file}"))
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_mac(name: &str) -> Option<String> {
    let mac = read_sys(name, "address")?.to_ascii_lowercase();
    if mac.is_empty() || mac == "00:00:00:00:00:00" {
        None
    } else {
        Some(mac)
    }
}

fn oper_up(name: &str) -> bool {
    match read_sys(name, "operstate").as_deref() {
        Some("up") | Some("unknown") => true,
        Some("down") => false,
        _ => read_sys(name, "flags").is_some_and(|f| {
            u32::from_str_radix(f.trim_start_matches("0x"), 16)
                .map(|bits| bits & 0x1 != 0)
                .unwrap_or(false)
        }),
    }
}

fn is_loopback_iface(name: &str) -> bool {
    name == "lo" || read_sys(name, "type").as_deref() == Some("772")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kernel_if_inet6_line() {
        let line = "fe800000000000006dd082fe420c6779 02 40 20 80     wlp191s0";
        let parsed = parse_if_inet6_line(line).expect("parse");
        assert_eq!(parsed.name, "wlp191s0");
        assert_eq!(parsed.index, 2);
        let expected: Ipv6Addr = "fe80::6dd0:82fe:420c:6779".parse().unwrap();
        assert_eq!(parsed.ip, expected);
        assert!(is_unicast_link_local(parsed.ip));
    }

    #[test]
    fn rejects_short_if_inet6() {
        assert!(parse_if_inet6_line("dead beef").is_none());
    }
}
