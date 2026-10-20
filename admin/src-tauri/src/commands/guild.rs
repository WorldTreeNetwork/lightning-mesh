// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Guild verbs. Signs identikey-log keyspace ops. Does not call
//! `apply_network_name` and does not stage `CLIENT_SSID`.

use crate::guild::{GuildReceipt, GuildSignInput, sign_guild_now};

#[tauri::command]
pub fn sign_guild_op(input: GuildSignInput) -> Result<GuildReceipt, String> {
    sign_guild_now(&input)
}
