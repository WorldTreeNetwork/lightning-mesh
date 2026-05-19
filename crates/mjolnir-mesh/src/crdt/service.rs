use std::collections::BTreeMap;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A mesh-wide service announcement (mDNS-style).
///
/// Keyed by service name at `/services/{name}`. Service expires when the
/// associated device lease (identified by `host_mac`) expires.
///
/// Uses `BTreeMap` instead of `HashMap` for deterministic serialization order,
/// which makes postcard round-trip equality straightforward.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub hostname: String,
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: String,
    pub txt: BTreeMap<String, String>,
    pub host_mac: [u8; 6],
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn postcard_roundtrip() {
        let mut txt = BTreeMap::new();
        txt.insert("path".to_string(), "/ipp/print".to_string());
        txt.insert("version".to_string(), "2.0".to_string());

        let original = ServiceEntry {
            hostname: "printer".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            port: 631,
            protocol: "_ipp._tcp".to_string(),
            txt,
            host_mac: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01],
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: ServiceEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original.hostname, decoded.hostname);
        assert_eq!(original.ip, decoded.ip);
        assert_eq!(original.port, decoded.port);
        assert_eq!(original.protocol, decoded.protocol);
        assert_eq!(original.txt, decoded.txt);
        assert_eq!(original.host_mac, decoded.host_mac);
    }
}
