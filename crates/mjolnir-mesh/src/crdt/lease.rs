use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::crdt::hlc::HLC;

/// A DHCP lease entry in the mesh CRDT.
///
/// Keyed by MAC address at `/devices/{mac}`. One entry per device.
/// Uses `IpAddr` (v4/v6 enum) for forward compatibility with IPv6.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;

    use super::*;

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
        assert_eq!(original.mac, decoded.mac);
        assert_eq!(original.ip, decoded.ip);
        assert_eq!(original.hostname, decoded.hostname);
        assert_eq!(original.router_id, decoded.router_id);
        assert_eq!(original.expiry, decoded.expiry);
        assert_eq!(original.hlc, decoded.hlc);
    }
}
