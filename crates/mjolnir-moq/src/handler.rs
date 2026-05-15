use moq_lite::OriginProducer;
use tracing::{info, warn};

use crate::iroh::endpoint::Connection;
use crate::session::SharedSessionMap;

/// Protocol handler for accepting incoming MoQ connections.
///
/// Shares the same session map and `OriginProducer` as the `MoqBridge` that created it.
/// Call `accept()` with an incoming iroh `Connection` to establish a MoQ session.
///
/// This handler uses the iroh version re-exported from `web_transport_iroh`
/// (currently iroh 0.96) to ensure compatibility with `Session::raw()`.
#[derive(Clone)]
pub struct MoqHandler {
    pub(crate) origin: OriginProducer,
    pub(crate) sessions: SharedSessionMap,
}

impl MoqHandler {
    /// Accept an incoming connection and run the MoQ session until it closes.
    ///
    /// This establishes a MoQ server session, stores it in the shared session map,
    /// then waits for the session to close before cleaning up.
    pub async fn accept(&self, connection: Connection) -> anyhow::Result<()> {
        let peer_id = connection.remote_id();
        info!(%peer_id, "accepting incoming MoQ connection");

        let wt_session = web_transport_iroh::Session::raw(connection);
        let moq_session = moq_lite::Server::new()
            .with_origin(self.origin.clone())
            .accept(wt_session.clone())
            .await?;

        let session = crate::MoqSession::new(wt_session, moq_session.clone());
        self.sessions.lock().await.insert(peer_id, session);
        info!(%peer_id, "incoming MoQ session established");

        // Wait for the session to close, then clean up.
        let _ = moq_session.closed().await;
        warn!(%peer_id, "incoming MoQ session closed");
        self.sessions.lock().await.remove(&peer_id);

        Ok(())
    }

    /// Return a reference to the shared origin.
    pub fn origin(&self) -> &OriginProducer {
        &self.origin
    }
}

impl std::fmt::Debug for MoqHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoqHandler")
            .field("sessions", &"SharedSessionMap")
            .finish_non_exhaustive()
    }
}
