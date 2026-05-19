pub mod encap;
pub mod iface;
pub mod link;

pub use encap::{spawn_encap_pair, DatagramConn, EncapError, EncapHandles};
pub use iface::{IfaceError, PeerInterface};
pub use link::{pick_link_31, LINK_BLOCK};
