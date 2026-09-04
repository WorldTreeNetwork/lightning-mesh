// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use futures_util::TryStreamExt;
use netlink_packet_route::neighbour::{NeighbourAddress, NeighbourAttribute, NeighbourState};
use netlink_packet_route::AddressFamily;

use super::types::{LinkLocalInterface, RawNeighbor};
use super::{format_mac, is_ula_or_link_local};

pub async fn dump_link_local(
    interfaces: &[LinkLocalInterface],
) -> Result<Vec<RawNeighbor>, String> {
    let (connection, handle, _) = rtnetlink::new_connection().map_err(|e| e.to_string())?;
    tokio::spawn(connection);

    let mut stream = handle
        .neighbours()
        .get()
        .set_address_family(AddressFamily::Inet6)
        .execute();

    let mut out = Vec::new();
    while let Some(msg) = stream.try_next().await.map_err(|e| e.to_string())? {
        if matches!(
            msg.header.state,
            NeighbourState::Failed | NeighbourState::Incomplete | NeighbourState::None
        ) {
            continue;
        }
        let ifindex = msg.header.ifindex;
        if interfaces
            .iter()
            .any(|i| i.index == ifindex && i.is_loopback)
        {
            continue;
        }

        let mut dest = None;
        let mut mac = None;
        for attr in msg.attributes {
            match attr {
                NeighbourAttribute::Destination(NeighbourAddress::Inet6(ip)) => dest = Some(ip),
                NeighbourAttribute::LinkLayerAddress(bytes) => mac = format_mac(&bytes),
                _ => {}
            }
        }
        let Some(address) = dest else {
            continue;
        };
        if !is_ula_or_link_local(address) {
            continue;
        }
        out.push(RawNeighbor {
            ifindex,
            address,
            mac,
            state: nud_label(msg.header.state),
            source: "neigh",
        });
    }
    Ok(out)
}

fn nud_label(state: NeighbourState) -> String {
    match state {
        NeighbourState::Reachable => "reachable".into(),
        NeighbourState::Stale => "stale".into(),
        NeighbourState::Delay => "delay".into(),
        NeighbourState::Probe => "probe".into(),
        NeighbourState::Permanent => "permanent".into(),
        NeighbourState::Noarp => "noarp".into(),
        other => other.to_string().to_ascii_lowercase(),
    }
}
