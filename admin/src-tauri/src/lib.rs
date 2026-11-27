// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use tauri::Manager;

mod canvas;
mod commands;
mod stdout;
pub mod app_tun;
pub mod guild;
pub mod guild_ceremony;
pub mod guild_view;
pub mod net;
pub mod permit;

use canvas::{close_canvas, list_canvases, open_canvas};
use commands::claim::{claim_owner_key, claim_prove, claim_request};
use commands::discovery::scan_link_local;
use commands::lan::fetch_lan_json;
use commands::guild::{guild_admit, guild_assign, guild_rename, guild_snapshot, sign_guild_op};
use commands::network_name::apply_network_name;
use commands::provision::{provision_node_health, provision_stock_node};
use stdout::fill_canvas_source;

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

    let builder = tauri::Builder::default()
        .manage(canvas::CanvasState::new())
        .setup(|app| {
            canvas::open_if_env(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                tauri::WindowEvent::Resized(_) => canvas::on_resized(app, window.label()),
                tauri::WindowEvent::Destroyed => canvas::on_destroyed(app, window.label()),
                _ => {}
            }
        })
        .plugin(tauri_plugin_opener::init());

    // Debug builds only. The socket stays on localhost. Release binaries do not start it.
    #[cfg(debug_assertions)]
    let builder = builder.plugin(
        tauri_plugin_mcp_bridge::Builder::new()
            .bind_address("127.0.0.1")
            .build(),
    );

    builder
        .invoke_handler(tauri::generate_handler![
            scan_link_local,
            fetch_lan_json,
            apply_network_name,
            provision_stock_node,
            provision_node_health,
            sign_guild_op,
            guild_snapshot,
            guild_rename,
            guild_assign,
            guild_admit,
            claim_owner_key,
            claim_request,
            claim_prove,
            open_canvas,
            close_canvas,
            list_canvases,
            fill_canvas_source
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
