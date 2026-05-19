use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A DNS entry in the mesh CRDT.
///
/// Keyed by hostname at `/dns/{hostname}`. Derived from the corresponding
/// `LeaseEntry` when a lease is written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsEntry {
    pub ip: IpAddr,
    pub mac: [u8; 6],
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn postcard_roundtrip() {
        let original = DnsEntry {
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
            mac: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: DnsEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original.ip, decoded.ip);
        assert_eq!(original.mac, decoded.mac);
    }
}
