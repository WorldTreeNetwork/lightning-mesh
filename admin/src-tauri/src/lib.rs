// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

mod commands;
mod guild;
pub mod net;

use commands::claim::{claim_owner_key, claim_prove, claim_request};
use commands::discovery::scan_link_local;
use commands::guild::sign_guild_op;
use commands::network_name::apply_network_name;

fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,lightning_admin=debug,lightning_admin_lib=debug"));
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_writer(std::io::stderr));
    let _ = registry.try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();
    tracing::info!(target: "startup", "lightning-admin starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_link_local,
            apply_network_name,
            sign_guild_op,
            claim_owner_key,
            claim_request,
            claim_prove
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
