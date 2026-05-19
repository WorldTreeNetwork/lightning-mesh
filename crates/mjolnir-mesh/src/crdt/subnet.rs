use ipnet::IpNet;
use serde::{Deserialize, Serialize};

use crate::crdt::hlc::HLC;

/// A subnet ownership claim in the mesh CRDT.
///
/// Keyed by CIDR (with `/` escaped to `_`) at `/subnets/{cidr}`.
/// Records which mesh node has claimed a given subnet range to prevent
/// two routers from claiming the same /24 at first boot.
///
/// This is NOT a routing table — Babel handles route computation and
/// installation. The claim exists only for first-boot coordination.
/// Conflicts resolve first-writer-wins on `claimed_at` HLC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubnetClaim {
    /// CIDR block claimed by this node (e.g. `10.42.1.0/24`).
    pub cidr: IpNet,
    pub owner_node_id: String,
    pub site_name: Option<String>,
    pub claimed_at: HLC,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn postcard_roundtrip() {
        let original = SubnetClaim {
            cidr: IpNet::from_str("10.42.1.0/24").unwrap(),
            owner_node_id: "router-b".to_string(),
            site_name: Some("office".to_string()),
            claimed_at: HLC {
                wall_clock: 1_700_000_001_000,
                counter: 0,
                node_id: "router-b".to_string(),
            },
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: SubnetClaim = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original.cidr, decoded.cidr);
        assert_eq!(original.owner_node_id, decoded.owner_node_id);
        assert_eq!(original.site_name, decoded.site_name);
        assert_eq!(original.claimed_at, decoded.claimed_at);
    }
}
