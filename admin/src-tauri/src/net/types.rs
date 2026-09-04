// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::net::Ipv6Addr;

use serde::{Deserialize, Serialize};

use super::{ipv6_scope, ipv6_to_base58, scoped_addr};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkLocalInterface {
    pub name: String,
    pub index: u32,
    pub mac: Option<String>,
    pub is_up: bool,
    pub is_loopback: bool,
    pub link_local: Vec<String>,
    pub unique_local: Vec<String>,
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
        let iface = iface.into();
        let scoped = if addr.is_unicast_link_local() {
            scoped_addr(addr, &iface)
        } else {
            addr.to_string()
        };
        Self {
            iface,
            ifindex,
            address: addr.to_string(),
            scoped,
            base58: ipv6_to_base58(addr),
            scope: ipv6_scope(addr).into(),
            mac,
            state: state.into(),
            kind: kind.into(),
            source: source.into(),
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
    pub address: Ipv6Addr,
    pub mac: Option<String>,
    pub state: String,
    pub source: &'static str,
}
