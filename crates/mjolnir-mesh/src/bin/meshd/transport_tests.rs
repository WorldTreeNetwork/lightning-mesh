use super::*;

#[tokio::test]
async fn strict_bind_clears_presets_and_never_falls_back() {
    for lan in [true, false] {
        let ep = build_endpoint(
            SecretKey::from_bytes(&[0x39; 32]),
            true,
            Some("[::1]:0".parse().unwrap()),
            lan,
            &[],
        )
        .await
        .unwrap();
        let sockets = ep.bound_sockets();
        assert_eq!(sockets.len(), 1);
        assert_eq!(sockets[0].ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
        ep.close().await;
        for bind in ["0.0.0.0:0", "[::]:0", "[fd81:2b22:8aca::dead]:49737"] {
            assert!(
                build_endpoint(
                    SecretKey::from_bytes(&[0x39; 32]),
                    true,
                    Some(bind.parse().unwrap()),
                    lan,
                    &[]
                )
                .await
                .is_err(),
                "{bind} lan={lan}"
            );
        }
        assert!(
            build_endpoint(SecretKey::from_bytes(&[0x39; 32]), true, None, lan, &[])
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn cancelling_a_session_aborts_its_tasks() {
    let tasks = SessionTasks::default();
    let handle = tasks.spawn(std::future::pending::<()>());
    drop(tasks);
    assert!(handle.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn no_guild_admits_a_relay_dial_and_refuses_a_direct_dial_with_no_route() {
    use iroh::endpoint::{BeforeConnectOutcome, EndpointHooks};
    let remote = EndpointAddr::new(SecretKey::from_bytes(&[0x41; 32]).public());
    let open = HouseDialGate {
        no_relay: false,
        guild: None,
        table: mjolnir_mesh::tun::underlay::OUTER_TABLE.into(),
        iface: MESH_BRIDGE.into(),
    };
    assert!(matches!(
        open.before_connect(&remote, TUN_ALPN).await,
        BeforeConnectOutcome::Accept
    ));
    let direct = HouseDialGate {
        no_relay: true,
        guild: None,
        table: "42881".into(),
        iface: MESH_BRIDGE.into(),
    };
    assert!(matches!(
        direct.before_connect(&remote, TUN_ALPN).await,
        BeforeConnectOutcome::Reject
    ));
}

/// All state belongs to this fixture: two veth ends, a dummy overlay, private
/// FIBs, exact policy rules, temporary configs and two supervised Babel children.
/// No namespaces, deployed services, inventory, guild files or static peer host
/// routes. Input-only local tables make same-netns packets actually traverse
/// the veth, rather than short-circuiting through the kernel's local table.
struct Fixture {
    names: [String; 3],
    tables: [&'static str; 4],
    rules: Vec<Vec<String>>,
    children: Vec<tokio::process::Child>,
    _dir: tempfile::TempDir,
    armed: bool,
}
impl Fixture {
    fn ip(args: &[&str]) -> String {
        let out = std::process::Command::new("ip")
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "ip {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }
    fn rule(&mut self, args: Vec<String>) {
        let mut command = vec!["-6", "rule", "add"];
        command.extend(args.iter().map(String::as_str));
        Self::ip(&command);
        self.rules.push(args);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for child in &mut self.children {
            let _ = child.start_kill();
        }
        for args in &self.rules {
            let mut command = vec!["-6", "rule", "del"];
            command.extend(args.iter().map(String::as_str));
            let _ = std::process::Command::new("ip").args(command).output();
        }
        for table in self.tables {
            let _ = std::process::Command::new("ip")
                .args(["-6", "route", "flush", "table", table])
                .output();
        }
        for name in [&self.names[0], &self.names[2]] {
            let _ = std::process::Command::new("ip")
                .args(["link", "del", name])
                .output();
        }
    }
}
fn hello(source: Ipv6Addr) -> Bytes {
    let mut p = vec![0; 60];
    p[0] = 0x60;
    p[6] = 17;
    p[4..6].copy_from_slice(&20u16.to_be_bytes());
    p[8..24].copy_from_slice(&source.octets());
    p[24..40].copy_from_slice(&mjolnir_mesh::tun::BABEL_MCAST.octets());
    p[40..42].copy_from_slice(&6696u16.to_be_bytes());
    p[42..44].copy_from_slice(&6696u16.to_be_bytes());
    p[44..46].copy_from_slice(&20u16.to_be_bytes());
    p[48..].copy_from_slice(&[42, 2, 0, 8, 4, 6, 0, 0, 0, 1, 0, 4]);
    Bytes::from(p)
}

#[tokio::test]
#[ignore = "requires CAP_NET_ADMIN, ip-full and babeld; same-netns veth only"]
async fn physical_babel_cold_start_and_established_outer_path_loss() {
    let mut fixture = Fixture {
        names: [
            format!("lmb{}a", std::process::id()),
            format!("lmb{}b", std::process::id()),
            format!("lmb{}o", std::process::id()),
        ],
        // Outer ids must fit babeld's 8-bit rtm_table (see OUTER_TABLE).
        // 211/212 stay off the daemon's own table so this test can flush
        // them. The local tables are iproute2-only and may be wide.
        tables: ["211", "212", "42873", "42874"],
        rules: Vec::new(),
        children: Vec::new(),
        _dir: tempfile::tempdir().unwrap(),
        armed: false,
    };
    // Refuse occupied names/tables rather than disturb an operator's state.
    for name in &fixture.names {
        assert!(!Path::new(&format!("/sys/class/net/{name}")).exists());
    }
    for table in fixture.tables {
        let out = std::process::Command::new("ip")
            .args(["-6", "route", "show", "table", table])
            .output()
            .unwrap();
        assert!(out.stdout.is_empty(), "fixture table {table} is occupied");
    }
    fixture.armed = true;
    let a = fixture.names[0].clone();
    let b = fixture.names[1].clone();
    let overlay = fixture.names[2].clone();
    Fixture::ip(&["link", "add", &a, "type", "veth", "peer", "name", &b]);
    Fixture::ip(&["link", "add", &overlay, "type", "dummy"]);
    let keys = [
        SecretKey::from_bytes(&[0x31; 32]),
        SecretKey::from_bytes(&[0x32; 32]),
    ];
    let nodes = [keys[0].public().to_string(), keys[1].public().to_string()];
    let hosts = nodes
        .each_ref()
        .map(|node| mjolnir_mesh::tun::router_ula(&[0x11; 32], node));
    for (i, iface) in [&a, &b].into_iter().enumerate() {
        Fixture::ip(&["link", "set", iface, "up"]);
        Fixture::ip(&[
            "-6",
            "addr",
            "add",
            &format!("{}/128", hosts[i]),
            "dev",
            iface,
        ]);
    }
    Fixture::ip(&["link", "set", &overlay, "up"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let body = std::fs::read_to_string("/proc/net/if_inet6").unwrap();
        if [&a, &b]
            .into_iter()
            .enumerate()
            .all(|(i, iface)| mjolnir_mesh::tun::house_addr_usable(&body, iface, hosts[i]))
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "fixture addresses did not pass DAD"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    const PORT: u16 = 49739;
    for (i, iface) in [&a, &b].into_iter().enumerate() {
        let host = format!("{}/128", hosts[i]);
        let local_table = fixture.tables[i + 2];
        let outer_table = fixture.tables[i];
        Fixture::ip(&[
            "-6", "route", "del", "local", &host, "dev", iface, "table", "local",
        ]);
        Fixture::ip(&[
            "-6",
            "route",
            "add",
            "local",
            &host,
            "dev",
            "lo",
            "table",
            local_table,
        ]);
        fixture.rule(
            [
                "pref",
                "11001",
                "iif",
                iface,
                "to",
                &host,
                "lookup",
                local_table,
            ]
            .map(String::from)
            .to_vec(),
        );
        Fixture::ip(&[
            "-6",
            "route",
            "add",
            "unreachable",
            "default",
            "table",
            outer_table,
            "metric",
            "42760",
        ]);
        fixture.rule(
            [
                "pref",
                "11002",
                "iif",
                "lo",
                "from",
                &host,
                "ipproto",
                "udp",
                "sport",
                &PORT.to_string(),
                "lookup",
                outer_table,
            ]
            .map(String::from)
            .to_vec(),
        );
        let conf = format!(
            "router-id 02:00:00:00:00:00:00:{:02x}\nexport-table {outer_table}\nimport-table 254\n{}",
            i + 1,
            render_babeld_conf(
                &BabelConfigInputs::new(None, &[])
                    .l2_interfaces(&[iface])
                    .house_router(Some(hosts[i]))
            )
        );
        let file = fixture._dir.path().join(format!("{i}.conf"));
        std::fs::write(&file, conf).unwrap();
        fixture.children.push(
            tokio::process::Command::new("babeld")
                .args(["-p", "45696", "-c"])
                .arg(&file)
                .arg("-I")
                .arg(file.with_extension("pid"))
                .arg("-S")
                .arg(file.with_extension("state"))
                .kill_on_drop(true)
                .spawn()
                .unwrap(),
        );
    }
    // No endpoint and no manually installed peer /128: only Babel can fill these tables.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let learned = [&a, &b].into_iter().enumerate().all(|(i, iface)| {
            let out = Fixture::ip(&["-6", "route", "show", "table", fixture.tables[i]]);
            physical_peer_route(&out, hosts[1 - i], iface)
        });
        if learned {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "physical Babel did not learn both host routes"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // Prefer an overlay route in main. Only the transport's source/port policy
    // may bypass it; ordinary inner house-host traffic must keep using it.
    for peer in hosts {
        Fixture::ip(&[
            "-6",
            "route",
            "add",
            &format!("{peer}/128"),
            "dev",
            &overlay,
            "metric",
            "1",
        ]);
    }
    let get = Fixture::ip(&[
        "-6",
        "route",
        "get",
        &hosts[1].to_string(),
        "from",
        &hosts[0].to_string(),
        "ipproto",
        "udp",
        "sport",
        &PORT.to_string(),
        "dport",
        &PORT.to_string(),
    ]);
    assert!(get.contains(&format!("dev {a}")), "{get}");
    let inner = Fixture::ip(&["-6", "route", "get", &hosts[1].to_string()]);
    assert!(inner.contains(&format!("dev {overlay}")), "{inner}");
    let mut endpoints = Vec::new();
    for (i, iface) in [&a, &b].into_iter().enumerate() {
        let gate = HouseDialGate {
            no_relay: true,
            guild: Some([0x11; 32]),
            table: fixture.tables[i].into(),
            iface: iface.to_string(),
        };
        let ep = build_endpoint_with_gate(
            keys[i].clone(),
            true,
            Some(SocketAddr::new(hosts[i].into(), PORT)),
            true,
            &[],
            gate,
        )
        .await
        .unwrap();
        assert_eq!(ep.bound_sockets().len(), 1);
        ep.set_alpns(vec![TUN_ALPN.to_vec()]);
        endpoints.push(ep);
    }
    let remote =
        EndpointAddr::new(keys[1].public()).with_ip_addr(SocketAddr::new(hosts[1].into(), PORT));
    let dial = endpoints[0].connect(remote, TUN_ALPN);
    let accept = async { endpoints[1].accept().await.unwrap().await.unwrap() };
    let (outgoing, incoming) = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::join!(dial, accept)
    })
    .await
    .unwrap();
    let outgoing = outgoing.unwrap();
    assert_eq!(outgoing.remote_id(), keys[1].public());
    let ll = mjolnir_mesh::tun::overlay_link_local(&nodes[1]);
    incoming.send_datagram(hello(ll)).unwrap();
    let got = tokio::time::timeout(Duration::from_secs(3), outgoing.read_datagram())
        .await
        .unwrap()
        .unwrap();
    let id = outgoing.stable_id() as u64;
    let mut neighbors = Neighbors::default();
    neighbors.register(
        id,
        Some(&"11".repeat(32)),
        &outgoing.remote_id().to_string(),
    );
    neighbors.observe(id, &got);
    let routes = NeighborRoutes::parse(
        &format!(
            "10.42.1.0/24 via inet6 {ll} dev mjolnir0 proto babel\nfd8f:78a9:2caa::2/128 via {ll} dev mjolnir0 proto babel"
        ),
        "mjolnir0",
    );
    assert_eq!(
        NeighborRoutes::default().select(ll.into(), &neighbors),
        Some(id)
    );
    for dest in ["10.42.1.50", "fd8f:78a9:2caa::2"] {
        let dest: IpAddr = dest.parse().unwrap();
        assert_eq!(routes.select(dest, &neighbors), Some(id));
        let mut packet = match dest {
            IpAddr::V4(v) => {
                let mut p = vec![0; 20];
                p[0] = 0x45;
                p[16..20].copy_from_slice(&v.octets());
                p
            }
            IpAddr::V6(v) => {
                let mut p = vec![0; 40];
                p[0] = 0x60;
                p[24..40].copy_from_slice(&v.octets());
                p
            }
        };
        let bytes = Bytes::from(std::mem::take(&mut packet));
        outgoing.send_datagram(bytes.clone()).unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), incoming.read_datagram())
                .await
                .unwrap()
                .unwrap(),
            bytes
        );
    }
    // Withdraw the physical host route while the connection and preferred
    // overlay route stay live. A kernel lookup must fail, and no outer payload
    // may leak into the overlay (or arrive on the established connection).
    fixture.children[0].kill().await.unwrap();
    let _ = fixture.children[0].wait().await;
    let route = Fixture::ip(&["-6", "route", "show", "table", fixture.tables[0]]);
    for line in route
        .lines()
        .filter(|line| line.starts_with(&hosts[1].to_string()))
    {
        let mut args = vec!["-6", "route", "del", "table", fixture.tables[0]];
        args.extend(line.split_whitespace());
        Fixture::ip(&args);
    }
    let absent = std::process::Command::new("ip")
        .args([
            "-6",
            "route",
            "get",
            &hosts[1].to_string(),
            "from",
            &hosts[0].to_string(),
            "ipproto",
            "udp",
            "sport",
            &PORT.to_string(),
            "dport",
            &PORT.to_string(),
        ])
        .output()
        .unwrap();
    assert!(!absent.status.success());
    outgoing
        .send_datagram(Bytes::from_static(b"must not escape on overlay"))
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(750), incoming.read_datagram())
            .await
            .is_err()
    );
    let inner = Fixture::ip(&["-6", "route", "get", &hosts[1].to_string()]);
    assert!(inner.contains(&format!("dev {overlay}")));
    neighbors.remove(id);
    assert_eq!(
        routes.select("10.42.1.50".parse().unwrap(), &neighbors),
        None
    );
    for ep in endpoints {
        ep.close().await;
    }
}
