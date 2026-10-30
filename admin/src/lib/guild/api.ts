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

export interface GuildSnapshot {
	actor: { id: string; hex: string } | null;
	guilds: Array<{
		name: string | null;
		workplace: boolean;
		profile: string;
		id: string;
		keyspaceId: string;
		outer: string | null;
		you: 'admin' | 'member' | 'absent' | 'no-seed';
		roster: Array<{ admin: boolean; actor: string }>;
		nodes: Array<{
			name: string | null;
			nodeId: string;
			accessId: string | null;
			accessNodeKeyspaceId: string | null;
		}>;
	}>;
	ops: Array<{ n: number; kind: string; id: string; hex: string; about?: string }>;
}

export async function guildSnapshot(): Promise<GuildSnapshot> {
	return invoke<GuildSnapshot>('guild_snapshot');
}

export async function guildRename(keyspaceId: string, name: string): Promise<void> {
	return invoke<void>('guild_rename', { keyspaceId, name });
}

export async function guildAssign(
	keyspaceId: string,
	nodeName: string,
	nodeId: string
): Promise<unknown> {
	return invoke<unknown>('guild_assign', { keyspaceId, nodeName, nodeId });
}
