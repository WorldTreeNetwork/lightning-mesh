// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Local guild log: look at a guild, and sign the ceremony that changes it.

use std::env;
use std::process::ExitCode;

use lightning_admin_lib::guild_ceremony::{
    config_dir, run_ceremony, CeremonyMode, CeremonyRequest,
};
use lightning_admin_lib::guild_view::{inspect, project, resolve_guild};
use serde_json::Value;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("lightning-guild: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();
    let cmd = args.first().map(String::as_str).unwrap_or("list");
    let config = config_dir()?;
    match cmd {
        "list" => {
            let view = inspect(&config)?;
            emit(json, &view, &render_list(&view));
        }
        "status" | "show" => {
            let query = args.get(1).map(String::as_str);
            let view = inspect(&config)?;
            let query = match query {
                Some(q) => q.to_string(),
                None => default_show(&view)?,
            };
            let id = resolve_guild(&config, &query)?;
            let one = project(&view, &id)?;
            emit(json, &one, &render_show(&one));
        }
        "assign" => {
            let guild = flag(&args, "--guild")?;
            let node_name = flag(&args, "--node-name")?;
            let node_id = flag(&args, "--node-id")?;
            let workplace_id = resolve_guild(&config, &guild)?;
            let value = run_ceremony(
                &config,
                &CeremonyRequest {
                    mode: CeremonyMode::Assign,
                    workplace_name: String::new(),
                    workplace_id,
                    node_name,
                    node_id,
                },
            )?;
            emit(json, &value, &value.to_string());
        }
        "ceremony" => {
            let workplace = flag(&args, "--workplace").unwrap_or_default();
            let workplace_id = flag(&args, "--workplace-id").unwrap_or_default();
            let node_name = flag(&args, "--node-name")?;
            let node_id = flag(&args, "--node-id")?;
            let mode = if workplace_id.is_empty() {
                CeremonyMode::Genesis
            } else {
                CeremonyMode::Assign
            };
            let value = run_ceremony(
                &config,
                &CeremonyRequest {
                    mode,
                    workplace_name: workplace,
                    workplace_id,
                    node_name,
                    node_id,
                },
            )?;
            emit(json, &value, &value.to_string());
        }
        "help" | "-h" | "--help" => {
            println!("{}", usage());
            return Ok(());
        }
        other => return Err(format!("unknown command {other}\n{}", usage())),
    }
    Ok(())
}

fn emit(json: bool, value: &Value, text: &str) {
    if json {
        println!("{value}");
    } else {
        println!("{text}");
    }
}

fn default_show(view: &Value) -> Result<String, String> {
    let guilds = view["guilds"].as_array().ok_or("no guilds")?;
    let workplaces: Vec<&Value> = guilds.iter().filter(|g| g["workplace"] == true).collect();
    match workplaces.len() {
        1 => Ok(workplaces[0]["keyspaceId"].as_str().unwrap().to_string()),
        0 => Err("name the guild to show".into()),
        _ => Err("more than one workplace; pass a name".into()),
    }
}

fn render_list(view: &Value) -> String {
    let mut out = String::new();
    if let Some(actor) = view["actor"].as_object() {
        out.push_str(&format!("actor  {}\n", actor["id"].as_str().unwrap_or("")));
    }
    out.push_str("held   local log, signatures verified\n");
    let Some(guilds) = view["guilds"].as_array() else {
        return out;
    };
    for g in guilds.iter().filter(|g| g["outer"].is_null()) {
        let name = g["name"].as_str().unwrap_or("(unnamed)");
        let id = g["id"].as_str().unwrap_or("");
        let you = g["you"].as_str().unwrap_or("");
        let nodes = g["nodes"].as_array().map(|n| n.len()).unwrap_or(0);
        let kind = if g["workplace"] == true { "workplace" } else { "house" };
        out.push_str(&format!("\n{name}\n  {kind}  you {you}  {nodes} node(s)\n  id  {id}\n"));
    }
    out
}

fn render_show(view: &Value) -> String {
    let g = &view["guild"];
    let mut out = String::new();
    let name = g["name"].as_str().unwrap_or("(unnamed)");
    out.push_str(&format!("{name}\n"));
    out.push_str(&format!("  profile  {}\n", g["profile"].as_str().unwrap_or("")));
    out.push_str(&format!("  you      {}\n", g["you"].as_str().unwrap_or("")));
    out.push_str("  held     local log, signatures verified\n");
    out.push_str(&format!("  id       {}\n", g["id"].as_str().unwrap_or("")));
    out.push_str(&format!("  hex      {}\n", g["keyspaceId"].as_str().unwrap_or("")));
    if let Some(actor) = view["actor"].as_object() {
        out.push_str(&format!("  actor    {}\n", actor["id"].as_str().unwrap_or("")));
    }
    out.push_str("\n  roster\n");
    if let Some(roster) = g["roster"].as_array() {
        for m in roster {
            let role = if m["admin"] == true { "admin" } else { "member" };
            out.push_str(&format!("    {role}  {}\n", m["actor"].as_str().unwrap_or("")));
        }
    }
    out.push_str("\n  nodes\n");
    if let Some(nodes) = g["nodes"].as_array() {
        if nodes.is_empty() {
            out.push_str("    (none)\n");
        }
        for n in nodes {
            let label = n["name"].as_str().unwrap_or("(unnamed)");
            out.push_str(&format!("    {label}\n"));
            out.push_str(&format!("      node    {}\n", n["nodeId"].as_str().unwrap_or("")));
            if let Some(access) = n["accessId"].as_str() {
                out.push_str(&format!("      access  {access}\n"));
            }
        }
    }
    if let Some(nested) = view["nested"].as_array() {
        if !nested.is_empty() {
            out.push_str("\n  nested\n");
            for n in nested {
                out.push_str(&format!(
                    "    {}  {}\n",
                    n["profile"].as_str().unwrap_or(""),
                    n["id"].as_str().unwrap_or("")
                ));
            }
        }
    }
    out.push_str("\n  ops\n");
    if let Some(ops) = view["ops"].as_array() {
        for op in ops {
            out.push_str(&format!(
                "    {:>2}  {:<22}  {}\n",
                op["n"].as_u64().unwrap_or(0),
                op["kind"].as_str().unwrap_or(""),
                op["id"].as_str().unwrap_or("")
            ));
        }
    }
    out.push_str("\n  id is base58 of the keyspace hash. A guild does not have an XID.\n");
    out
}

fn flag(args: &[String], name: &str) -> Result<String, String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
        .ok_or_else(|| format!("missing {name}\n{}", usage()))
}

fn usage() -> String {
    "\
lightning-guild                       list guilds on this machine
lightning-guild show [name|id]        one guild, its roster, nodes, and ops
lightning-guild assign --guild NAME --node-name NAME --node-id HEX
lightning-guild ceremony --workplace NAME --node-name NAME --node-id HEX
lightning-guild ceremony --workplace-id HEX --node-name NAME --node-id HEX

--json prints the same data as one JSON object.
The guild id is base58 of the keyspace content hash. It is not an XID.
Ops stay in the local log. They are not pushed to a router."
        .into()
}
