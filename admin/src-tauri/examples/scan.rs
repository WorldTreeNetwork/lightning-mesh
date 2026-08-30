// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

#[tokio::main]
async fn main() {
    let scan = lightning_admin_lib::net::scan_link_local()
        .await
        .expect("scan_link_local");
    println!(
        "{}",
        serde_json::to_string_pretty(&scan).expect("json")
    );
}
