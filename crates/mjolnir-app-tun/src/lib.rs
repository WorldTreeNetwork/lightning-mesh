// SPDX-License-Identifier: AGPL-3.0-or-later
//! House address derivation and a caller-supplied datagram transport on one app TUN.
//! This crate does not dial, link Iroh, or read a guild log.
mod address;
mod encap;
mod session;

pub use address::*;
pub use encap::{DatagramConn, EncapError, EncapHandles, spawn_encap_pair};
pub use session::{AppTunOpen, open_app_tun};
