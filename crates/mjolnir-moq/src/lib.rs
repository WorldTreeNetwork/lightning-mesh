//! mjolnir-moq: Bridge between iroh QUIC connections and moq-lite sessions.
//!
//! This crate provides the thin protocol layer that wraps iroh `Connection`s
//! as WebTransport sessions consumable by moq-lite's publish/subscribe API.
//!
//! ## iroh version note
//!
//! This crate re-exports iroh types from `web_transport_iroh::iroh` (currently iroh 0.96)
//! to ensure type compatibility with `web_transport_iroh::Session::raw()`.
//! Use `mjolnir_moq::iroh` when working with MoQ connections.

mod handler;
mod session;

pub use handler::MoqHandler;
pub use session::{MoqBridge, MoqSession};

// Re-export the iroh version used by web-transport-iroh for type compatibility.
pub use session::iroh;

/// ALPN protocol identifier for MoQ over iroh.
///
/// Uses the moq-lite ALPN string encoded as bytes for iroh's ALPN matching.
pub const MOQ_ALPN: &[u8] = moq_lite::ALPN_LITE.as_bytes();
