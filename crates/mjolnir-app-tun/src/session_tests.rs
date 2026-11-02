use super::*;
use crate::EncapError;
use bytes::Bytes;
use std::collections::BTreeSet;
use std::io;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};

const FOREIGN: [u8; 6] = [0xfd, 0x8f, 0x78, 0xa9, 0x2c, 0xaa];
const OWN: [u8; 6] = [0xfd, 0x81, 0x2b, 0x22, 0x8a, 0xca];

#[derive(Default)]
struct RouteState {
    addresses: BTreeSet<String>,
    routes: BTreeSet<String>,
    commands: Vec<Vec<String>>,
    fail_delete: bool,
    fail_observe: bool,
    skip_delete: bool,
    hide_routes: bool,
}

#[derive(Default)]
struct TestNetwork(Mutex<RouteState>);
impl Network for TestNetwork {
    fn ip(&self, args: &[&str]) -> Result<(), ()> {
        let mut state = self.0.lock().unwrap();
        state
            .commands
            .push(args.iter().map(|s| s.to_string()).collect());
        match args {
            ["-6", "route", "replace", prefix, "dev", _] => {
                state.routes.insert(prefix.to_string());
            }
            ["-6", "route", "del", prefix, "dev", _] => {
                if state.fail_delete {
                    return Err(());
                }
                if !state.skip_delete {
                    state.routes.remove(*prefix);
                }
            }
            ["-6", "addr", "del", address, "dev", _] => {
                state.addresses.remove(*address);
            }
            ["link", "del", _] => {}
            _ => panic!("unexpected command: {args:?}"),
        }
        Ok(())
    }

    fn route_present(&self, _dev: &str, prefix: &[u8; 6]) -> Result<bool, ()> {
        let state = self.0.lock().unwrap();
        if state.fail_observe {
            return Err(());
        }
        Ok(!state.hide_routes && state.routes.contains(&format!("{}/48", prefix48(prefix))))
    }
}

struct Release(Arc<AtomicBool>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct ReadProbe {
    close: oneshot::Receiver<()>,
    started: Arc<AtomicBool>,
    _release: Release,
}
impl AsyncRead for ReadProbe {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.started.store(true, Ordering::SeqCst);
        Pin::new(&mut self.close).poll(cx).map(|_| Ok(()))
    }
}
struct WriteProbe(Release);
impl AsyncWrite for WriteProbe {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let _ = &self.0;
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Clone)]
struct ConnProbe {
    close: Arc<tokio::sync::Mutex<oneshot::Receiver<()>>>,
    started: Arc<AtomicBool>,
    released: Arc<AtomicBool>,
}
#[async_trait::async_trait]
impl DatagramConn for ConnProbe {
    async fn send_datagram(&self, _: Bytes) -> Result<(), EncapError> {
        Ok(())
    }
    async fn recv_datagram(&self) -> Result<Bytes, EncapError> {
        let _release = Release(Arc::clone(&self.released));
        self.started.store(true, Ordering::SeqCst);
        let _ = (&mut *self.close.lock().await).await;
        Err(EncapError::ConnectionClosed)
    }
}

struct Probes {
    read_started: Arc<AtomicBool>,
    recv_started: Arc<AtomicBool>,
    read_released: Arc<AtomicBool>,
    recv_released: Arc<AtomicBool>,
    write_released: Arc<AtomicBool>,
    tx: tokio::task::AbortHandle,
    rx: tokio::task::AbortHandle,
    read_close: oneshot::Sender<()>,
    recv_close: oneshot::Sender<()>,
}
fn packet_tasks() -> (EncapHandles, Probes) {
    let read_started = Arc::new(AtomicBool::new(false));
    let recv_started = Arc::new(AtomicBool::new(false));
    let read_released = Arc::new(AtomicBool::new(false));
    let recv_released = Arc::new(AtomicBool::new(false));
    let write_released = Arc::new(AtomicBool::new(false));
    let (read_close, read_rx) = oneshot::channel();
    let (recv_close, recv_rx) = oneshot::channel();
    let handles = spawn_encap_pair(
        ReadProbe {
            close: read_rx,
            started: read_started.clone(),
            _release: Release(read_released.clone()),
        },
        WriteProbe(Release(write_released.clone())),
        ConnProbe {
            close: Arc::new(tokio::sync::Mutex::new(recv_rx)),
            started: recv_started.clone(),
            released: recv_released.clone(),
        },
        MTU,
    );
    let probes = Probes {
        read_started,
        recv_started,
        read_released,
        recv_released,
        write_released,
        tx: handles.tx.abort_handle(),
        rx: handles.rx.abort_handle(),
        read_close,
        recv_close,
    };
    (handles, probes)
}

fn owned() -> (Owned, Arc<TestNetwork>) {
    let local = app_ula(&[0x11; 32], &[0xab; 32]);
    let peer = app_ula(&[0x11; 32], &[0xcd; 32]);
    let network = Arc::new(TestNetwork::default());
    {
        let mut state = network.0.lock().unwrap();
        state.addresses.insert(format!("{local}/128"));
        state.routes.insert(format!("{peer}/128"));
    }
    (
        Owned {
            name: "mja-test".into(),
            local,
            peer,
            guild: [0x11; 32],
            owned_prefixes: Vec::new(),
            network: network.clone(),
        },
        network,
    )
}
fn open(handles: EncapHandles) -> (AppTunOpen, Arc<TestNetwork>) {
    let (owned, network) = owned();
    let address = Some(owned.local.to_string());
    (
        AppTunOpen {
            address,
            up: true,
            inner: Some(Session::start(owned, handles)),
        },
        network,
    )
}
async fn wait_for(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(2), async {
        while !predicate() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("tasks did not release their resources");
}
fn assert_clean(network: &TestNetwork) {
    let state = network.0.lock().unwrap();
    assert!(state.addresses.is_empty(), "owned address left behind");
    assert!(
        state.routes.is_empty(),
        "owned peer or admitted route left behind"
    );
    assert!(
        state
            .commands
            .iter()
            .any(|args| args == &["link", "del", "mja-test"])
    );
}
async fn assert_released(probes: &Probes) {
    wait_for(|| {
        probes.tx.is_finished()
            && probes.rx.is_finished()
            && probes.read_released.load(Ordering::SeqCst)
            && probes.recv_released.load(Ordering::SeqCst)
            && probes.write_released.load(Ordering::SeqCst)
    })
    .await;
}

#[tokio::test]
async fn tun_eof_aborts_pending_receive_and_cleans_all_owned_state() {
    let (handles, probes) = packet_tasks();
    let (mut opened, network) = open(handles);
    opened.sync_admitted_routes(&[OWN, FOREIGN]).unwrap();
    wait_for(|| {
        probes.read_started.load(Ordering::SeqCst) && probes.recv_started.load(Ordering::SeqCst)
    })
    .await;
    // Keep the receive-close sender alive: only session cancellation can end it.
    let _ = &probes.recv_close;
    probes.read_close.send(()).unwrap();
    // Move the sender separately without losing the resource observers.
    wait_for(|| {
        probes.tx.is_finished()
            && probes.rx.is_finished()
            && probes.recv_released.load(Ordering::SeqCst)
            && probes.read_released.load(Ordering::SeqCst)
            && probes.write_released.load(Ordering::SeqCst)
            && !opened.inner.as_ref().unwrap().up.load(Ordering::SeqCst)
    })
    .await;
    assert!(!opened.is_up());
    assert_eq!(
        opened.address.as_deref(),
        Some("fd81:2b22:8aca:1:bd5e:fa7a:5460:697")
    );
    assert_clean(&network);
}

#[tokio::test]
async fn receive_close_aborts_pending_tun_read_and_cleans_all_owned_state() {
    let (handles, probes) = packet_tasks();
    let (mut opened, network) = open(handles);
    opened.sync_admitted_routes(&[FOREIGN]).unwrap();
    wait_for(|| {
        probes.read_started.load(Ordering::SeqCst) && probes.recv_started.load(Ordering::SeqCst)
    })
    .await;
    let _ = &probes.read_close;
    probes.recv_close.send(()).unwrap();
    wait_for(|| {
        probes.tx.is_finished()
            && probes.rx.is_finished()
            && probes.read_released.load(Ordering::SeqCst)
            && probes.recv_released.load(Ordering::SeqCst)
            && probes.write_released.load(Ordering::SeqCst)
            && !opened.inner.as_ref().unwrap().up.load(Ordering::SeqCst)
    })
    .await;
    assert!(!opened.is_up());
    assert_clean(&network);
}

#[tokio::test(flavor = "current_thread")]
async fn drop_before_supervisor_polls_aborts_both_pending_tasks_and_cleans_routes() {
    let (handles, probes) = packet_tasks();
    wait_for(|| {
        probes.read_started.load(Ordering::SeqCst) && probes.recv_started.load(Ordering::SeqCst)
    })
    .await;
    let (mut opened, network) = open(handles);
    opened.sync_admitted_routes(&[FOREIGN]).unwrap();
    assert!(opened.is_up());
    let watch = opened.inner.as_ref().unwrap().watch.abort_handle();
    // No await since Session::start: this current-thread executor cannot have
    // polled the spawned supervisor, while both packet tasks are already pending.
    drop(opened);
    assert_clean(&network);
    assert_released(&probes).await;
    wait_for(|| watch.is_finished()).await;
}

#[test]
fn prefix_sync_excludes_own_house_and_retries_failed_deletes_and_observations() {
    let (mut owned, network) = owned();
    owned.sync(&[OWN, FOREIGN, FOREIGN]).unwrap();
    assert_eq!(owned.owned_prefixes, vec![FOREIGN]);
    assert!(
        !network
            .0
            .lock()
            .unwrap()
            .routes
            .contains("fd81:2b22:8aca::/48")
    );
    assert!(
        !network
            .0
            .lock()
            .unwrap()
            .routes
            .contains("fd12:3456:789a::/48")
    );
    network.0.lock().unwrap().fail_delete = true;
    assert!(owned.sync(&[]).is_err());
    assert_eq!(owned.owned_prefixes, vec![FOREIGN]);
    network.0.lock().unwrap().fail_delete = false;
    owned.sync(&[]).unwrap();
    assert!(owned.owned_prefixes.is_empty());

    network.0.lock().unwrap().fail_observe = true;
    assert!(owned.sync(&[FOREIGN]).is_err());
    assert_eq!(owned.owned_prefixes, vec![FOREIGN]);
    network.0.lock().unwrap().skip_delete = true;
    assert!(owned.sync(&[]).is_err());
    assert_eq!(owned.owned_prefixes, vec![FOREIGN]);
    network.0.lock().unwrap().skip_delete = false;
    // An actual deletion with an uncertain observation still stays owned.
    assert!(owned.sync(&[]).is_err());
    assert_eq!(owned.owned_prefixes, vec![FOREIGN]);
    network.0.lock().unwrap().fail_observe = false;
    owned.sync(&[]).unwrap();
    assert!(owned.owned_prefixes.is_empty());
    assert!(
        !network
            .0
            .lock()
            .unwrap()
            .routes
            .contains("fd8f:78a9:2caa::/48")
    );
}

#[test]
fn confirmed_absence_does_not_claim_a_prefix() {
    let (mut owned, network) = owned();
    network.0.lock().unwrap().hide_routes = true;
    assert!(owned.sync(&[FOREIGN]).is_err());
    assert!(owned.owned_prefixes.is_empty());
}
