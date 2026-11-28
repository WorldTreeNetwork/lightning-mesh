// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use iroh::{Endpoint, RelayMode, endpoint::presets, protocol::Router};
use iroh_gossip::{Gossip, TopicId};
use mjolnir_mesh::crdt::{gossip::GossipMessage, hlc::tick_hlc, service::ServiceEntryV2};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time::timeout,
};

#[tokio::test]
async fn observer_reports_a_service_from_a_ticket_only_peer() -> Result<()> {
    observe_service(false).await
}

#[tokio::test]
async fn json_consumer_observes_full_messages_and_completion() -> Result<()> {
    observe_service(true).await
}

async fn observe_service(json_output: bool) -> Result<()> {
    let publisher = Endpoint::builder(presets::Minimal)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")?
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await?;
    let owner = publisher.id().to_string();
    let ticket = data_encoding::BASE32_NOPAD.encode(&postcard::to_allocvec(&publisher.addr())?);
    let gossip = Gossip::builder().spawn(publisher.clone());
    let router = Router::builder(publisher)
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();
    let topic = TopicId::from_bytes(*blake3::hash(b"mjolnir/mesh/crdt/v0").as_bytes());
    let mut channel = gossip.subscribe(topic, vec![]).await?;

    let dir = tempfile::tempdir()?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_mjolnir-node"));
    command
        .current_dir(dir.path())
        .args(["--lan", "--peer", &ticket, "listen"]);
    if json_output {
        command.args(["--json", "--count", "1"]);
    }
    let mut observer = command
        .env("RUST_LOG", if json_output { "off" } else { "info" })
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut lines = BufReader::new(observer.stdout.take().unwrap()).lines();
    let stamp = tick_hlc(&owner);
    let service = GossipMessage::ServicePublishV2 {
        name: "edgebox-test".into(),
        entry: ServiceEntryV2 {
            owner_node_id: owner.clone(),
            first_claimed_at: stamp.clone(),
            updated_at: stamp,
            ip: "127.0.0.1".parse()?,
            port: 8080,
            protocol: "tcp".into(),
            txt: Default::default(),
            host_mac: None,
        },
    };
    timeout(Duration::from_secs(20), async {
        channel.joined().await?;
        if json_output {
            channel.broadcast(vec![255].into()).await?;
            let mut events = Vec::new();
            while let Some(line) = lines.next_line().await? {
                let row: serde_json::Value = serde_json::from_str(&line)?;
                assert_eq!(row["version"], 1);
                assert!(row["time_ms"].is_u64());
                events.push(row["event"].as_str().unwrap().to_owned());
                if row["event"] == "decode_error" {
                    assert_eq!(row["data"]["delivered_from"], owner);
                    break;
                }
            }
            assert_eq!(
                events,
                ["identity", "listening", "neighbor_up", "decode_error"]
            );
        }
        channel
            .broadcast(postcard::to_allocvec(&service)?.into())
            .await?;
        while let Some(line) = lines.next_line().await? {
            if json_output {
                let row: serde_json::Value = serde_json::from_str(&line)?;
                assert_eq!(row["event"], "message");
                assert_eq!(row["data"]["count"], 1);
                assert_eq!(row["data"]["delivered_from"], owner);
                assert_eq!(row["data"]["message"], serde_json::to_value(&service)?);
                let last = lines.next_line().await?.context("missing stopped record")?;
                let last: serde_json::Value = serde_json::from_str(&last)?;
                assert_eq!(last["event"], "stopped");
                assert_eq!(
                    last["data"],
                    serde_json::json!({"reason": "count", "count": 1})
                );
                assert!(lines.next_line().await?.is_none());
                return Ok(());
            }
            if line.contains("service publish v2")
                && line.contains("edgebox-test")
                && line.contains(&owner)
            {
                return Ok(());
            }
        }
        anyhow::bail!("observer exited before reporting the service")
    })
    .await
    .context("timed out waiting for observed service")??;
    if json_output {
        assert!(observer.wait().await?.success());
    } else {
        observer.kill().await?;
    }
    router.shutdown().await?;
    Ok(())
}
