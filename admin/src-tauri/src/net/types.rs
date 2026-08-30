// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::net::Ipv6Addr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkLocalInterface {
    pub name: String,
    pub index: u32,
    pub mac: Option<String>,
    pub is_up: bool,
    pub is_loopback: bool,
    pub link_local: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkLocalNeighbor {
    pub iface: String,
    pub ifindex: u32,
    pub address: String,
    pub scoped: String,
    pub mac: Option<String>,
    pub state: String,
    pub kind: String,
    pub source: String,
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
