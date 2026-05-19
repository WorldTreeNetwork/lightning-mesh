//! IP-packet encap/decap between a per-peer TUN interface and an Iroh QUIC connection.
//!
//! Spawns two long-lived tasks per peer:
//!   - `tun_to_iroh`: read IP frames from the TUN, send each as one Iroh datagram.
//!   - `iroh_to_tun`: receive datagrams from the connection, write each to the TUN.
//!
//! The two tasks live as long as the Iroh `Connection`. When the connection
//! closes (peer disconnect, error), both tasks observe the error on their next
//! send/recv and return — cleanly.

use bytes::Bytes;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Trait abstracting a per-peer Iroh connection just enough that we can
/// unit-test the encap loops without needing a real iroh::Connection.
#[async_trait::async_trait]
pub trait DatagramConn: Send + Sync + 'static {
    async fn send_datagram(&self, packet: Bytes) -> Result<(), EncapError>;
    async fn recv_datagram(&self) -> Result<Bytes, EncapError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EncapError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("connection closed")]
    ConnectionClosed,
    #[error("datagram too large: {0} bytes")]
    DatagramTooLarge(usize),
}

/// Spawn the two encap tasks. Returns join handles so the caller can await
/// them or abort on disconnect.
///
/// `tun_read` reads IP frames; `tun_write` writes IP frames. They are typically
/// the read/write halves of the same TUN device.
pub fn spawn_encap_pair<R, W, C>(
    mut tun_read: R,
    mut tun_write: W,
    conn: C,
    mtu: usize,
) -> EncapHandles
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    C: DatagramConn + Clone,
{
    let conn_a = conn.clone();
    let tx = tokio::spawn(async move {
        let mut buf = vec![0u8; mtu];
        loop {
            let n = match tun_read.read(&mut buf).await {
                Ok(0) => return Ok(()), // TUN closed
                Ok(n) => n,
                Err(e) => return Err(EncapError::from(e)),
            };
            let packet = Bytes::copy_from_slice(&buf[..n]);
            conn_a.send_datagram(packet).await?;
        }
    });

    let rx = tokio::spawn(async move {
        loop {
            let packet = match conn.recv_datagram().await {
                Ok(p) => p,
                Err(EncapError::ConnectionClosed) => return Ok(()),
                Err(e) => return Err(e),
            };
            if let Err(e) = tun_write.write_all(&packet).await {
                return Err(EncapError::from(e));
            }
        }
    });

    EncapHandles { tx, rx }
}

pub struct EncapHandles {
    pub tx: tokio::task::JoinHandle<Result<(), EncapError>>,
    pub rx: tokio::task::JoinHandle<Result<(), EncapError>>,
}

impl EncapHandles {
    /// Abort both encap tasks. Idempotent.
    pub fn abort(&self) {
        self.tx.abort();
        self.rx.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[derive(Clone)]
    struct MockConn {
        tx: Arc<mpsc::Sender<Bytes>>,
        rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Bytes>>>,
    }

    impl MockConn {
        fn pair() -> (MockConn, MockConn) {
            let (a_tx, b_rx) = mpsc::channel::<Bytes>(256);
            let (b_tx, a_rx) = mpsc::channel::<Bytes>(256);
            let a = MockConn {
                tx: Arc::new(a_tx),
                rx: Arc::new(tokio::sync::Mutex::new(a_rx)),
            };
            let b = MockConn {
                tx: Arc::new(b_tx),
                rx: Arc::new(tokio::sync::Mutex::new(b_rx)),
            };
            (a, b)
        }
    }

    #[async_trait::async_trait]
    impl DatagramConn for MockConn {
        async fn send_datagram(&self, packet: Bytes) -> Result<(), EncapError> {
            self.tx
                .send(packet)
                .await
                .map_err(|_| EncapError::ConnectionClosed)
        }

        async fn recv_datagram(&self) -> Result<Bytes, EncapError> {
            self.rx
                .lock()
                .await
                .recv()
                .await
                .ok_or(EncapError::ConnectionClosed)
        }
    }

    #[tokio::test]
    async fn tun_to_iroh_forwards_one_packet() {
        let (conn_a, conn_b) = MockConn::pair();
        let (tun_write_a, tun_read_a) = tokio::io::duplex(1500);

        // We only need the tx task; provide a dummy write half for the rx side.
        let (_dummy_write, dummy_read) = tokio::io::duplex(1500);
        let handles = spawn_encap_pair(tun_read_a, dummy_read, conn_a, 1500);

        // Write a fake IP packet into the tun write end.
        let packet = Bytes::from_static(&[0x45, 0x00, 0x00, 0x14, 0x00, 0x01]);
        {
            let mut w = tun_write_a;
            tokio::io::AsyncWriteExt::write_all(&mut w, &packet)
                .await
                .unwrap();
        }

        // The packet should arrive on conn_b's recv side.
        let received = conn_b.rx.lock().await.recv().await.unwrap();
        assert_eq!(received, packet);

        handles.abort();
    }

    #[tokio::test]
    async fn iroh_to_tun_forwards_one_packet() {
        let (conn_a, conn_b) = MockConn::pair();
        let (tun_write_a, mut tun_read_out) = tokio::io::duplex(1500);

        // Dummy read half for the tx side.
        let (_dummy_write, dummy_read) = tokio::io::duplex(1500);
        let handles = spawn_encap_pair(dummy_read, tun_write_a, conn_a, 1500);

        // Push a packet through the connection.
        let packet = Bytes::from_static(&[0x45, 0x00, 0x00, 0x14, 0xAB, 0xCD]);
        conn_b.tx.send(packet.clone()).await.unwrap();

        // Should appear on the tun write end.
        let mut buf = vec![0u8; packet.len()];
        tokio::io::AsyncReadExt::read_exact(&mut tun_read_out, &mut buf)
            .await
            .unwrap();
        assert_eq!(buf, packet.as_ref());

        handles.abort();
    }

    // NOTE: a many-packet loopback test against `tokio::io::duplex` would conflate
    // semantics: duplex is a byte-stream so multiple small writes coalesce, while
    // a real TUN device preserves packet boundaries (one read() = one IP packet).
    // The unit tests above (single-packet forward in each direction) exercise the
    // task wiring; multi-packet behavior under real TUN message semantics is
    // covered by the integration test in US-009.

    #[tokio::test]
    async fn abort_stops_tasks() {
        let (conn_a, _conn_b) = MockConn::pair();
        let (tun_write, tun_read) = tokio::io::duplex(1500);
        let (_dummy_write, dummy_read) = tokio::io::duplex(1500);

        let handles = spawn_encap_pair(tun_read, dummy_read, conn_a, 1500);
        handles.abort();

        // Both handles should resolve as cancelled (JoinError::is_cancelled).
        let tx_result = handles.tx.await;
        let rx_result = handles.rx.await;
        assert!(tx_result.unwrap_err().is_cancelled());
        assert!(rx_result.unwrap_err().is_cancelled());

        drop(tun_write);
    }
}
