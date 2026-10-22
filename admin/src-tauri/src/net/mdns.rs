// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Short Avahi browse. Hostnames are attached to addresses we already saw.
//! A resolved address that is not in the neighbor table still becomes a row,
//! so a host that only answered mDNS is visible.

use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

#[derive(Clone, Debug)]
pub struct MdnsRecord {
    pub iface: String,
    pub hostname: String,
    pub address: String,
    pub mac: Option<String>,
}

pub async fn browse() -> Vec<MdnsRecord> {
    let child = match Command::new("avahi-browse")
        .args(["-artpk"])
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Vec::new(),
    };
    let output = match timeout(Duration::from_millis(1500), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        _ => return Vec::new(),
    };
    parse_parsable(&String::from_utf8_lossy(&output.stdout))
}

pub fn parse_parsable(text: &str) -> Vec<MdnsRecord> {
    let mut out = Vec::new();
    for line in text.lines() {
        if !line.starts_with('=') {
            continue;
        }
        let parts: Vec<&str> = line.split(';').collect();
        // = ; iface ; proto ; name ; type ; domain ; hostname ; address ; port ; txt
        if parts.len() < 8 {
            continue;
        }
        let hostname = parts[6].trim();
        let address = parts[7].trim();
        if hostname.is_empty() || address.is_empty() || hostname == address {
            continue;
        }
        let mac = mac_in_name(parts[3]);
        let rec = MdnsRecord {
            iface: parts[1].to_string(),
            hostname: hostname.trim_end_matches('.').to_string(),
            address: address.to_string(),
            mac,
        };
        if out.iter().any(|r: &MdnsRecord| r.address == rec.address && r.hostname == rec.hostname)
        {
            continue;
        }
        out.push(rec);
    }
    out
}

fn mac_in_name(name: &str) -> Option<String> {
    let start = name.rfind('[')?;
    let end = name.rfind(']')?;
    if end <= start + 1 {
        return None;
    }
    let mac = &name[start + 1..end];
    let bytes: Vec<_> = mac.split(':').collect();
    if bytes.len() == 6 && bytes.iter().all(|b| b.len() == 2) {
        Some(mac.to_ascii_lowercase())
    } else {
        None
    }
}

pub fn apply_names(neighbors: &mut Vec<super::types::LinkLocalNeighbor>, names: &[MdnsRecord]) {
    use std::net::IpAddr;

    for name in names {
        let Ok(ip) = name.address.parse::<IpAddr>() else {
            continue;
        };
        let mut matched = false;
        for row in neighbors.iter_mut() {
            if row.address == name.address
                || row.address.split('%').next() == Some(name.address.as_str())
            {
                if row.hostname.is_none() {
                    row.hostname = Some(name.hostname.clone());
                }
                if row.mac.is_none() {
                    row.mac = name.mac.clone();
                }
                matched = true;
            }
        }
        if matched {
            continue;
        }
        let mut row = super::types::LinkLocalNeighbor::from_ip(
            name.iface.clone(),
            0,
            ip,
            name.mac.clone(),
            "announced",
            "neighbor",
            "mdns",
        );
        row.hostname = Some(name.hostname.clone());
        neighbors.push(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolved_workstation_line() {
        let text = "=;enp0;IPv4;edgebox [98:FE:54:08:25:FC];Workstation;local;edgebox.local;10.42.61.119;9;\n";
        let rows = parse_parsable(text);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].hostname, "edgebox.local");
        assert_eq!(rows[0].address, "10.42.61.119");
        assert_eq!(rows[0].mac.as_deref(), Some("98:fe:54:08:25:fc"));
    }
}
