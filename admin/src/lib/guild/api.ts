import { invoke } from '@tauri-apps/api/core';

export type GuildVerb = 'join' | 'change' | 'assign-node' | 'nest';

export interface GuildReceipt {
	kind: string;
	keyspaceId: string;
	contentHash: string;
	actor: string;
	logPath: string;
}

export interface GuildSignInput {
	verb: GuildVerb;
	actorSeedHex: string;
	guildIdHex?: string;
	nodeIdHex?: string;
}

export async function signGuildOp(input: GuildSignInput): Promise<GuildReceipt> {
	return invoke<GuildReceipt>('sign_guild_op', { input });
}
