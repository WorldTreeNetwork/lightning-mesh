//! Backhaul endpoint lifetime and an independent physical Babel underlay.
//! The endpoint binds the identity ULA. A house guild is not required, and a
//! peer in another guild is dialed at `ula_addr` of its endpoint id.
use super::*;
use mjolnir_mesh::tun::underlay::{
    OUTER_MARK, OUTER_TABLE, on_link_backhaul_route, physical_peer_route, relay_routes,
};

/// Tokio handles detach on drop. Keep explicit cancellation ownership for every
/// task in one endpoint session, including tasks started before an early error.
#[derive(Default)]
pub(super) struct SessionTasks(Mutex<Vec<tokio::task::AbortHandle>>);
impl SessionTasks {
    pub(super) fn spawn<F>(&self, f: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = tokio::spawn(f);
        self.0.lock().unwrap().push(handle.abort_handle());
        handle
    }
}
impl Drop for SessionTasks {
    fn drop(&mut self) {
        for task in self.0.get_mut().unwrap().drain(..) {
            task.abort();
        }
    }
}

/// Dial address for a peer. Computed from the endpoint id alone.
pub(super) fn peer_backhaul_addr(peer: &str) -> Ipv6Addr {
    mjolnir_mesh::tun::ula_addr(peer)
}

#[derive(Debug)]
pub(super) struct HouseDialGate {
    pub(super) no_relay: bool,
    pub(super) guild: Option<[u8; 32]>,
    pub(super) table: String,
    pub(super) iface: String,
}
impl HouseDialGate {
    pub(super) fn configured(no_relay: bool) -> Self {
        let guild = std::fs::read_to_string(mjolnir_mesh::tun::HOUSE_GUILD_PATH)
            .ok()
            .and_then(|s| mjolnir_mesh::tun::parse_house_guild_id(&s));
        Self {
            no_relay,
            guild,
            table: OUTER_TABLE.into(),
            iface: MESH_BRIDGE.into(),
        }
    }
}
impl iroh::endpoint::EndpointHooks for HouseDialGate {
    async fn before_connect(
        &self,
        remote: &EndpointAddr,
        _alpn: &[u8],
    ) -> iroh::endpoint::BeforeConnectOutcome {
        use iroh::endpoint::BeforeConnectOutcome::{Accept, Reject};
        // Relays are the off-link path. A missing guild must not refuse them.
        if !self.no_relay {
            return Accept;
        }
        let id = remote.id.to_string();
        let backhaul = mjolnir_mesh::tun::ula_addr(&id);
        let house = self
            .guild
            .map(|guild| mjolnir_mesh::tun::router_ula(&guild, &id));
        let routes = tokio::process::Command::new("ip")
            .args(["-6", "route", "show", "table", &self.table])
            .output()
            .await;
        let reachable = routes.is_ok_and(|out| {
            if !out.status.success() {
                return false;
            }
            let text = String::from_utf8_lossy(&out.stdout);
            on_link_backhaul_route(&text, backhaul, &self.iface)
                || physical_peer_route(&text, backhaul, &self.iface)
                || house.is_some_and(|peer| physical_peer_route(&text, peer, &self.iface))
        });
        if reachable { Accept } else { Reject }
    }
}

fn binding_witness(node: &str) -> Option<(Ipv6Addr, u32)> {
    let kernel = std::fs::read_to_string("/proc/net/if_inet6").ok()?;
    let index = std::fs::read_to_string(format!("/sys/class/net/{MESH_BRIDGE}/ifindex"))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    mjolnir_mesh::tun::underlay::usable_backhaul_binding(node, &kernel, MESH_BRIDGE, index)
}

pub(super) async fn supervised_mesh(cli: Cli, secret: SecretKey) -> Result<()> {
    let Command::Mesh {
        roster,
        peer,
        babel_config,
        client_iface,
        backhaul_iface,
        lan_tunnels,
        claims_file,
        directory_file,
        spool_dir,
        overlay,
        gateway,
        name,
    } = cli.command
    else {
        unreachable!()
    };
    anyhow::ensure!(
        backhaul_iface == MESH_BRIDGE,
        "house transport requires --backhaul-iface br-mesh"
    );
    if let Some(bind) = cli.bind {
        anyhow::ensure!(!bind.ip().is_unspecified(), "wildcard --bind is forbidden");
    }
    let lan = cli.lan || !(cli.internet || !cli.relay.is_empty());
    let no_relay = cli.no_relay || lan;
    let node = secret.public().to_string();
    // Kept only for the legacy directory/inventory column, never assigned,
    // bound, dialed, claimed, or used to index a connection.
    let legacy_column = mjolnir_mesh::tun::backhaul_addr(&node);
    let tasks = SessionTasks::default();
    tasks.spawn(reconcile_backhaul_addr(
        MESH_BRIDGE.to_owned(),
        legacy_column,
        mjolnir_mesh::tun::ula_addr(&node),
        node.clone(),
    ));
    tasks.spawn(led_status_loop(
        MESH_BRIDGE.to_owned(),
        mjolnir_mesh::tun::ula_addr(&node),
    ));
    // Starts before any endpoint/dial, and remains running while the house
    // address is absent. Its peer routes are learned, never synthesized.
    tasks.spawn(physical_babel(
        node.clone(),
        claims_file.clone(),
        babel_config.with_extension("underlay.conf"),
        gateway,
    ));
    let node_name = name
        .or_else(uci_node_name)
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty());
    let mut guard = None;
    loop {
        let Some(witness) = binding_witness(&node) else {
            tokio::select! { _ = tokio::signal::ctrl_c() => return Ok(()), _ = tokio::time::sleep(Duration::from_secs(1)) => {} }
            continue;
        };
        if guard.is_none() {
            match OuterGuard::install(!no_relay).await {
                Ok(g) => guard = Some(g),
                Err(e) => {
                    warn!("outer route guard unavailable; endpoint stays closed: {e:#}");
                    tokio::select! { _ = tokio::signal::ctrl_c() => return Ok(()), _ = tokio::time::sleep(Duration::from_secs(3)) => {} }
                    continue;
                }
            }
        }
        let bind = cli
            .bind
            .unwrap_or(SocketAddr::new(IpAddr::V6(witness.0), MESH_IROH_PORT));
        if let Some(g) = &mut guard {
            g.bind_route(bind).await?;
        }
        let endpoint =
            match build_endpoint(secret.clone(), no_relay, Some(bind), lan, &cli.relay).await {
                Ok(ep) => ep,
                Err(e) => {
                    warn!(%bind,"house bind failed without fallback: {e:#}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
        if let Some(g) = &mut guard {
            if let Some(actual) = endpoint.bound_sockets().first() {
                if let Err(e) = g.bind_route(*actual).await {
                    endpoint.close().await;
                    return Err(e);
                }
            }
        }
        // Recheck after binding: DAD, guild replacement and interface loss may
        // have happened during builder initialization.
        if binding_witness(&node) != Some(witness) {
            endpoint.close().await;
            continue;
        }
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let session = run_mesh(
            endpoint.clone(),
            no_relay,
            roster.clone(),
            peer.clone(),
            babel_config.clone(),
            client_iface.clone(),
            lan,
            lan_tunnels,
            Some(MESH_BRIDGE.to_owned()),
            claims_file.clone(),
            directory_file.clone(),
            spool_dir.clone(),
            overlay,
            gateway,
            node_name.clone(),
            legacy_column,
            load_claims(&claims_file),
            stop_rx,
        );
        tokio::pin!(session);
        let mut poll = tokio::time::interval(Duration::from_secs(1));
        let quit = loop {
            tokio::select! {
                result = &mut session => { endpoint.close().await; return result; }
                _ = tokio::signal::ctrl_c() => { break true; }
                _ = poll.tick() => {
                    if binding_witness(&node) != Some(witness) { break false; }
                    if let Some(g) = &mut guard {
                        if let Err(e) = g.refresh().await { warn!("underlay route refresh failed: {e:#}"); break false; }
                    }
                }
            }
        };
        // Close first, then explicitly cancel all session tasks and keys before
        // the supervisor is allowed to bind a replacement endpoint.
        endpoint.close().await;
        let _ = stop_tx.send(true);
        session.await?;
        if quit {
            return Ok(());
        }
    }
}

/// A separate Babel instance has no overlay neighbor from which to learn a
/// recursive peer path. Its port and state are independent of procd Babel.
async fn physical_babel(node: String, claims_file: PathBuf, config: PathBuf, gateway: bool) {
    let mut child: Option<tokio::process::Child> = None;
    let mut applied = String::new();
    let mut admitted = AdmittedPrefixes::default();
    loop {
        let claims = load_claims(&claims_file);
        let subnet = claims
            .values()
            .filter(|c| c.owner_node_id == node)
            .find_map(|c| match c.cidr {
                IpNet::V4(n) if !mjolnir_mesh::tun::in_backhaul_block(&n) => Some(n),
                _ => None,
            });
        if let Some(prefixes) = admitted.read_admitted_prefixes() {
            let conf = format!(
                "export-table {OUTER_TABLE}\nimport-table 254\n{}",
                render_babeld_conf(
                    &BabelConfigInputs::new(subnet, &[])
                        .l2_interfaces(&[MESH_BRIDGE])
                        .house_router(known_house_router(&node))
                        .admitted(&prefixes)
                        .gateway(gateway && local_egress().await.is_some_and(|e| e.healthy))
                )
            );
            let alive = child
                .as_mut()
                .is_some_and(|c| matches!(c.try_wait(), Ok(None)));
            if conf != applied || !alive {
                if let Some(mut old) = child.take() {
                    let _ = old.kill().await;
                    let _ = old.wait().await;
                }
                // SIGKILL cannot withdraw kernel routes. Clear only the
                // physical Babel routes in our private table before restart;
                // never flush main or rely on BusyBox's proto selector.
                for family in ["-4", "-6"] {
                    if let Ok(out) = tokio::process::Command::new("ip")
                        .args([family, "route", "show", "table", OUTER_TABLE])
                        .output()
                        .await
                    {
                        if out.status.success() {
                            for line in String::from_utf8_lossy(&out.stdout).lines() {
                                let fields: Vec<_> = line.split_whitespace().collect();
                                if fields.windows(2).any(|p| p == ["proto", "babel"])
                                    && fields.windows(2).any(|p| p == ["dev", MESH_BRIDGE])
                                {
                                    let mut args =
                                        vec![family, "route", "del", "table", OUTER_TABLE];
                                    args.extend(fields);
                                    let _ = ip(&args).await;
                                }
                            }
                        }
                    }
                }
                if let Some(parent) = config.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match write_atomic_if_changed(&config, &conf) {
                    Ok(_) => {
                        match tokio::process::Command::new("babeld")
                            .args(["-p", "6697", "-c"])
                            .arg(&config)
                            .arg("-I")
                            .arg(config.with_extension("pid"))
                            .arg("-S")
                            .arg(config.with_extension("state"))
                            .kill_on_drop(true)
                            .spawn()
                        {
                            Ok(c) => {
                                child = Some(c);
                                applied = conf;
                            }
                            Err(e) => warn!("physical Babel start failed: {e}"),
                        }
                    }
                    Err(e) => warn!("physical Babel config failed: {e}"),
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Route locally generated transport packets through a separate FIB. Forwarded
/// inner packets have no socket/cgroup match and keep using the normal FIB.
/// cgroup v2 plus nft socket matching also covers Iroh's independently-created
/// relay TCP sockets; binding only the UDP source would not protect those.
struct OuterGuard {
    cgroup: Option<PathBuf>,
    original: Option<PathBuf>,
    routes: [Vec<Vec<String>>; 2],
    binding: Option<SocketAddr>,
}
async fn ip(args: &[&str]) -> Result<()> {
    let out = tokio::process::Command::new("ip")
        .args(args)
        .output()
        .await?;
    anyhow::ensure!(
        out.status.success(),
        "ip {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}
impl OuterGuard {
    async fn install(relays: bool) -> Result<Self> {
        let (cgroup, original) = if relays {
            let body = std::fs::read_to_string("/proc/self/cgroup")?;
            let current = body
                .lines()
                .find_map(|l| l.strip_prefix("0::"))
                .context("cgroup v2 is required for relay route isolation")?;
            let original = Path::new("/sys/fs/cgroup").join(current.trim_start_matches('/'));
            let cgroup = original.join(format!("mjolnir-transport-{}", std::process::id()));
            std::fs::create_dir_all(&cgroup).context("creating transport cgroup")?;
            (Some(cgroup), Some(original))
        } else {
            (None, None)
        };
        let mut guard = Self {
            cgroup,
            original,
            routes: Default::default(),
            binding: None,
        };
        // Install fail-closed terminal routes before the marking/rules. Main
        // WAN defaults can carry relay traffic, but never substitute for a
        // missing physical host route in the reserved house address space.
        for family in ["-4", "-6"] {
            let prefix = if family == "-4" {
                "10.254.0.0/16"
            } else {
                "fc00::/7"
            };
            ip(&[
                family,
                "route",
                "replace",
                "unreachable",
                prefix,
                "table",
                OUTER_TABLE,
                "metric",
                "42760",
            ])
            .await?;
            ip(&[
                family,
                "route",
                "replace",
                "unreachable",
                "default",
                "table",
                OUTER_TABLE,
                "metric",
                "42760",
            ])
            .await?;
        }
        guard.ensure_on_link().await?;
        guard.refresh().await?;
        for family in ["-4", "-6"] {
            // Remove only our exact rule, including leftovers from a crash.
            let _ = ip(&[
                family,
                "rule",
                "del",
                "pref",
                "12000",
                "fwmark",
                OUTER_MARK,
                "lookup",
                OUTER_TABLE,
            ])
            .await;
            ip(&[
                family,
                "rule",
                "add",
                "pref",
                "12000",
                "fwmark",
                OUTER_MARK,
                "lookup",
                OUTER_TABLE,
            ])
            .await?;
        }
        if let Some(cgroup) = &guard.cgroup {
            let relative = cgroup
                .strip_prefix("/sys/fs/cgroup")?
                .to_string_lossy()
                .trim_start_matches('/')
                .to_owned();
            let level = relative.split('/').count();
            let rule = format!(
                "table inet mjolnir_transport {{ chain output {{ type route hook output priority mangle; policy accept; socket cgroupv2 level {level} \"{relative}\" meta l4proto tcp meta mark set {OUTER_MARK}; socket cgroupv2 level {level} \"{relative}\" udp sport != {{ 6696, 6697, 5353 }} meta mark set {OUTER_MARK}; }} }}\n"
            );
            let _ = tokio::process::Command::new("nft")
                .args(["delete", "table", "inet", "mjolnir_transport"])
                .output()
                .await;
            use std::process::Stdio;
            let mut cmd = tokio::process::Command::new("nft")
                .args(["-f", "-"])
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;
            cmd.stdin.take().unwrap().write_all(rule.as_bytes()).await?;
            let out = cmd.wait_with_output().await?;
            anyhow::ensure!(
                out.status.success(),
                "nft transport guard: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            std::fs::write(cgroup.join("cgroup.procs"), std::process::id().to_string())
                .context("moving daemon into transport cgroup")?;
        }
        Ok(guard)
    }
    // Native UDP must have a route at the initial lookup, before nft's output
    // hook. An exact source/port rule distinguishes outer UDP from forwarded
    // inner house-host packets, which retain the ordinary Babel FIB.
    async fn bind_route(&mut self, bind: SocketAddr) -> Result<()> {
        if self.binding == Some(bind) {
            return Ok(());
        }
        let family = if bind.is_ipv4() { "-4" } else { "-6" };
        let source = bind.ip().to_string();
        let port = bind.port().to_string();
        let mut args = vec![
            family, "rule", "add", "pref", "11999", "iif", "lo", "from", &source, "ipproto", "udp",
        ];
        if bind.port() != 0 {
            args.extend(["sport", &port]);
        }
        args.extend(["lookup", OUTER_TABLE]);
        if self.binding.is_none() {
            let mut delete = args.clone();
            delete[2] = "del";
            while ip(&delete).await.is_ok() {}
        }
        ip(&args).await?;
        if let Some(old) = self.binding {
            let family = if old.is_ipv4() { "-4" } else { "-6" };
            let source = old.ip().to_string();
            let port = old.port().to_string();
            let mut args = vec![
                family, "rule", "del", "pref", "11999", "iif", "lo", "from", &source, "ipproto",
                "udp",
            ];
            if old.port() != 0 {
                args.extend(["sport", &port]);
            }
            args.extend(["lookup", OUTER_TABLE]);
            ip(&args).await?;
        }

        self.binding = Some(bind);
        Ok(())
    }

    async fn refresh(&mut self) -> Result<()> {
        for (slot, family) in ["-4", "-6"].into_iter().enumerate() {
            let out = tokio::process::Command::new("ip")
                .args([family, "route", "show", "table", "main"])
                .output()
                .await?;
            anyhow::ensure!(out.status.success(), "reading main underlay routes failed");
            let next = relay_routes(&String::from_utf8_lossy(&out.stdout));
            // Never flush by protocol: BusyBox ip ignores that selector.
            for old in &self.routes[slot] {
                if !next.contains(old) {
                    let mut args = vec![family, "route", "del", "table", OUTER_TABLE];
                    args.extend(old.iter().map(String::as_str));
                    let _ = ip(&args).await;
                }
            }
            for new in &next {
                let mut args = vec![family, "route", "replace", "table", OUTER_TABLE];
                args.extend(new.iter().map(String::as_str));
                ip(&args).await?;
            }
            self.routes[slot] = next;
        }
        self.ensure_on_link().await?;
        Ok(())
    }

    /// Identity `/64` on `br-mesh` beats `unreachable fc00::/7`. Peers are
    /// on-link. This is not a per-peer static host route.
    async fn ensure_on_link(&self) -> Result<()> {
        let prefix = mjolnir_mesh::tun::ula_link_prefix().to_string();
        ip(&[
            "-6",
            "route",
            "replace",
            &prefix,
            "dev",
            MESH_BRIDGE,
            "scope",
            "link",
            "table",
            OUTER_TABLE,
            "metric",
            "256",
        ])
        .await
    }
}
impl Drop for OuterGuard {
    fn drop(&mut self) {
        // Endpoint sessions are closed before this guard is released. A stale
        // FIB/rule is harmless without the uniquely owned output marking rule.
        if self.cgroup.is_some() {
            let _ = std::process::Command::new("nft")
                .args(["delete", "table", "inet", "mjolnir_transport"])
                .output();
        }
        if let Some(bind) = self.binding {
            let family = if bind.is_ipv4() { "-4" } else { "-6" };
            let mut args = vec![
                family.to_owned(),
                "rule".into(),
                "del".into(),
                "pref".into(),
                "11999".into(),
                "iif".into(),
                "lo".into(),
                "from".into(),
                bind.ip().to_string(),
                "ipproto".into(),
                "udp".into(),
            ];
            if bind.port() != 0 {
                args.extend(["sport".into(), bind.port().to_string()]);
            }
            args.extend(["lookup".into(), OUTER_TABLE.into()]);
            let _ = std::process::Command::new("ip").args(args).output();
        }
        if let Some(original) = &self.original {
            let _ = std::fs::write(
                original.join("cgroup.procs"),
                std::process::id().to_string(),
            );
        }
        if let Some(cgroup) = &self.cgroup {
            let _ = std::fs::remove_dir(cgroup);
        }
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
