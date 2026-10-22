// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::net::{IpAddr, Ipv6Addr};

use serde::{Deserialize, Serialize};

use super::{ipv6_to_base58, scoped_addr};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkLocalInterface {
    pub name: String,
    pub index: u32,
    pub mac: Option<String>,
    pub is_up: bool,
    pub is_loopback: bool,
    pub link_local: Vec<String>,
    pub unique_local: Vec<String>,
    pub ipv4: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkLocalNeighbor {
    pub iface: String,
    pub ifindex: u32,
    pub address: String,
    pub scoped: String,
    /// Bitcoin-alphabet base58 of the 16 IPv6 octets (one hex couplet = one byte).
    pub base58: String,
    /// `unique-local` (`fc00::/7`) or `link-local` (`fe80::/10`).
    pub scope: String,
    pub mac: Option<String>,
    pub state: String,
    pub kind: String,
    pub source: String,
    /// Kernel `NTF_ROUTER`: this neighbor is a router on the link.
    pub router: bool,
    /// mDNS hostname (`edgebox.local`), when Avahi resolved this address.
    pub hostname: Option<String>,
    /// `name` from `GET /api/node` when this address is a Lightning node.
    pub mesh_name: Option<String>,
    pub mesh_node_id: Option<String>,
    pub mesh_subnet: Option<String>,
    pub mesh_backhaul: Option<String>,
}

impl LinkLocalNeighbor {
    pub fn from_addr(
        iface: impl Into<String>,
        ifindex: u32,
        addr: Ipv6Addr,
        mac: Option<String>,
        state: impl Into<String>,
        kind: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::from_ip(iface, ifindex, IpAddr::V6(addr), mac, state, kind, source)
    }

    pub fn from_ip(
        iface: impl Into<String>,
        ifindex: u32,
        addr: IpAddr,
        mac: Option<String>,
        state: impl Into<String>,
        kind: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let iface = iface.into();
        let (address, scoped, base58, scope) = match addr {
            IpAddr::V6(v6) => {
                let scoped = if v6.is_unicast_link_local() {
                    scoped_addr(v6, &iface)
                } else {
                    v6.to_string()
                };
                (
                    v6.to_string(),
                    scoped,
                    ipv6_to_base58(v6),
                    super::ipv6_scope(v6).to_string(),
                )
            }
            IpAddr::V4(v4) => (v4.to_string(), v4.to_string(), String::new(), "ipv4".into()),
        };
        Self {
            iface,
            ifindex,
            address,
            scoped,
            base58,
            scope,
            mac,
            state: state.into(),
            kind: kind.into(),
            source: source.into(),
            router: false,
            hostname: None,
            mesh_name: None,
            mesh_node_id: None,
            mesh_subnet: None,
            mesh_backhaul: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    pub interfaces: Vec<LinkLocalInterface>,
    pub neighbors: Vec<LinkLocalNeighbor>,
    pub probed: bool,
    pub probe_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawNeighbor {
    pub ifindex: u32,
    pub address: IpAddr,
    pub mac: Option<String>,
    pub state: String,
    pub source: &'static str,
    pub router: bool,
}
