pub mod encap;
pub mod fib;
pub mod iface;
pub mod link;
pub mod mcast;
pub mod neighbor;
pub mod overlay;
pub mod underlay;

pub use encap::{DatagramConn, EncapError, EncapHandles, spawn_encap_pair};
pub use fib::Fib;
#[cfg(target_os = "linux")]
pub use iface::spawn_overlay_tun;
pub use iface::{
    IfaceError, OVERLAY_IFACE, OverlayLink, PeerInterface, TUNNEL_MTU, Tunnel, spawn_tunnel,
};
pub use link::{
    BACKHAUL_PREFIX_LEN, HOUSE_APP_SUBNET, HOUSE_GUILD_PATH, HOUSE_ROUTER_SUBNET, HOUSE_ULA_DOMAIN,
    HOUSE_ULA_STATE_PATH, HouseIntent, HouseStateAction, IFA_F_DADFAILED, IFA_F_TENTATIVE,
    IfInet6Addr, LINK_BLOCK, ULA_ID_INPUT, ULA_PREFIX_LEN, app_ula, backhaul_addr,
    backhaul_addr_missing, backhaul_addr_salted, desired_router_ula, host_route_usable,
    house_addr_usable, house_after, house_intent, house_ula_prefix, in_backhaul_block,
    is_unique_local, parse_house_guild_id, parse_house_state, parse_if_inet6, pick_link_31,
    published_house_ula, router_ula, ula_addr, ula_link_prefix, ula_prefix,
};
pub use mcast::{BABEL_MCAST, BABEL_PORT, OverlayDest, classify, is_babel_multicast};
pub use overlay::{OverlayHandles, UnicastRouter, spawn_overlay, spawn_overlay_routed};

pub use neighbor::{NeighborRoutes, Neighbors, overlay_link_local};
