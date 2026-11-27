// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Read a node's LAN HTTP documents. This does not SSH and does not write radio settings.

use crate::net::http_get_json;
use serde_json::Value;

#[tauri::command]
pub async fn fetch_lan_json(address: String, path: String, scope_id: Option<u32>) -> Result<Value, String> {
    let scope = scope_id.unwrap_or(0);
    tokio::task::spawn_blocking(move || http_get_json(&address, scope, &path))
        .await
        .map_err(|e| e.to_string())?
}
