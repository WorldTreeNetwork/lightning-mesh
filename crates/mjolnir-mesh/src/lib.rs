pub mod crdt;

pub use crdt::{
    dns::DnsEntry,
    gossip::GossipMessage,
    hlc::HLC,
    lease::LeaseEntry,
    service::ServiceEntry,
    subnet::SubnetClaim,
};
