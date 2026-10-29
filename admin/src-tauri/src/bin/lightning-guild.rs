// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Run the guild ceremony in `admin/scripts/guild/ceremony.rhai`.

use std::env;
use std::process::ExitCode;

use lightning_admin_lib::guild_ceremony::{
    config_dir, run_ceremony, CeremonyMode, CeremonyRequest,
};

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("lightning-guild: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<serde_json::Value, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("ceremony") {
        return Err(usage());
    }
    let mut workplace = String::new();
    let mut workplace_id = String::new();
    let mut node_name = String::new();
    let mut node_id = String::new();
    let mut i = 1;
    while i < args.len() {
        let val = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {}", args[i]))?;
        match args[i].as_str() {
            "--workplace" => workplace = val.clone(),
            "--workplace-id" => workplace_id = val.clone(),
            "--node-name" => node_name = val.clone(),
            "--node-id" => node_id = val.clone(),
            other => return Err(format!("unknown argument {other}\n{}", usage())),
        }
        i += 2;
    }
    if node_name.is_empty() || node_id.is_empty() {
        return Err(usage());
    }
    let mode = if workplace_id.is_empty() {
        CeremonyMode::Genesis
    } else {
        CeremonyMode::Assign
    };
    let req = CeremonyRequest {
        mode,
        workplace_name: workplace,
        workplace_id,
        node_name,
        node_id,
    };
    run_ceremony(&config_dir()?, &req)
}

fn usage() -> String {
    "usage:\n  \
lightning-guild ceremony --workplace NAME --node-name NAME --node-id HEX\n  \
lightning-guild ceremony --workplace-id HEX --node-name NAME --node-id HEX\n\n\
Genesis (empty guild log only) signs an Admin house guild, a workplace house\n\
guild, assign-node, and an access-node nest. A workplace id assigns another\n\
node onto that guild. Names go in guilds.json. The seed stays in actor-seed."
        .into()
}
