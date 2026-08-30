// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Winbox-style first screen: every IPv6 link-local neighbor on any local
//! interface. Mirrors Papyrus's `list_vms` / `ping_box` shape — a thin Tauri
//! command over a native scan, `invoke`d from a Svelte store.

use crate::net::{ScanResult, scan_link_local as scan};

/// Enumerate IPv6 link-local addresses on every interface, dump the kernel
/// neighbor table, and probe `ff02::1` (all-nodes) per up non-loopback iface.
///
/// Never errors on an empty LAN — that is a valid empty `neighbors` list.
/// Probe permission failures are reported in `probe_error` and do not fail
/// the command; the neighbor-table dump still returns.
#[tauri::command]
pub async fn scan_link_local() -> Result<ScanResult, String> {
    scan().await
}
