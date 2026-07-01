pub mod encap;
pub mod iface;
pub mod link;
pub mod mcast;
pub mod overlay;

pub use encap::{spawn_encap_pair, DatagramConn, EncapError, EncapHandles};
pub use iface::{
    overlay_link_local, spawn_tunnel, IfaceError, OverlayLink, PeerInterface, Tunnel,
    OVERLAY_IFACE, TUNNEL_MTU,
};
#[cfg(target_os = "linux")]
pub use iface::spawn_overlay_tun;
pub use link::{backhaul_addr, pick_link_31, BACKHAUL_PREFIX_LEN, LINK_BLOCK};
pub use mcast::{classify, is_babel_multicast, OverlayDest, BABEL_MCAST, BABEL_PORT};
pub use overlay::{spawn_overlay, OverlayHandles};
