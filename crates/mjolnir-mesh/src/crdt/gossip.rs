use serde::{Deserialize, Serialize};

use crate::crdt::{dns::DnsEntry, hlc::HLC, lease::LeaseEntry, service::ServiceEntry, subnet::SubnetClaim};

/// Wire message enum for CRDT gossip replication.
///
/// All variants are serialized with postcard; the enum discriminant is a
/// single byte prefix. Gossip is best-effort; the CRDT merge function
/// handles duplicates, reordering, and lost messages correctly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    LeaseUpdate(LeaseEntry),
    LeaseRelease {
        mac: [u8; 6],
        hlc: HLC,
    },
    DnsUpdate {
        hostname: String,
        entry: DnsEntry,
    },
    ServiceUpdate {
        name: String,
        entry: ServiceEntry,
    },
    SubnetClaimUpdate {
        cidr: String,
        entry: SubnetClaim,
    },
    SubnetClaimRelease {
        cidr: String,
        hlc: HLC,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::net::{IpAddr, Ipv4Addr};
    use std::str::FromStr;

    use ipnet::IpNet;

    use super::*;

    fn make_hlc(wall_clock: u64, counter: u32, node_id: &str) -> HLC {
        HLC {
            wall_clock,
            counter,
            node_id: node_id.to_string(),
        }
    }

    #[test]
    fn postcard_roundtrip_lease_update() {
        let msg = GossipMessage::LeaseUpdate(LeaseEntry {
            mac: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
            hostname: Some("laptop".to_string()),
            router_id: "router-a".to_string(),
            expiry: 1_700_000_000,
            hlc: make_hlc(1_700_000_000_000, 0, "router-a"),
        });
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: GossipMessage = postcard::from_bytes(&bytes).unwrap();
        // Compare via serialized bytes — LeaseEntry fields don't all impl Eq
        assert_eq!(postcard::to_allocvec(&msg).unwrap(), postcard::to_allocvec(&decoded).unwrap());
    }

    #[test]
    fn postcard_roundtrip_lease_release() {
        let msg = GossipMessage::LeaseRelease {
            mac: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
            hlc: make_hlc(1_700_000_000_000, 1, "router-b"),
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: GossipMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, postcard::to_allocvec(&decoded).unwrap());
    }

    #[test]
    fn postcard_roundtrip_dns_update() {
        let msg = GossipMessage::DnsUpdate {
            hostname: "laptop".to_string(),
            entry: DnsEntry {
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
                mac: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            },
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: GossipMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, postcard::to_allocvec(&decoded).unwrap());
    }

    #[test]
    fn postcard_roundtrip_service_update() {
        let mut txt = BTreeMap::new();
        txt.insert("path".to_string(), "/ipp/print".to_string());

        let msg = GossipMessage::ServiceUpdate {
            name: "printer._ipp._tcp".to_string(),
            entry: ServiceEntry {
                hostname: "printer".to_string(),
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
                port: 631,
                protocol: "_ipp._tcp".to_string(),
                txt,
                host_mac: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01],
            },
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: GossipMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, postcard::to_allocvec(&decoded).unwrap());
    }

    #[test]
    fn postcard_roundtrip_subnet_claim_update() {
        let msg = GossipMessage::SubnetClaimUpdate {
            cidr: "10.42.1.0_24".to_string(),
            entry: SubnetClaim {
                cidr: IpNet::from_str("10.42.1.0/24").unwrap(),
                owner_node_id: "router-c".to_string(),
                site_name: None,
                claimed_at: make_hlc(1_700_000_002_000, 0, "router-c"),
            },
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: GossipMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, postcard::to_allocvec(&decoded).unwrap());
    }

    #[test]
    fn postcard_roundtrip_subnet_claim_release() {
        let msg = GossipMessage::SubnetClaimRelease {
            cidr: "10.42.1.0_24".to_string(),
            hlc: make_hlc(1_700_000_003_000, 0, "router-c"),
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: GossipMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, postcard::to_allocvec(&decoded).unwrap());
    }
}
