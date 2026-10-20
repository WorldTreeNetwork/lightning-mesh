import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

import { invoke } from '@tauri-apps/api/core';
import { GUILD_LABEL, GuildStore } from './store.svelte';
import type { GuildReceipt } from './api';

const SEED = '11'.repeat(32);
const GUILD = '22'.repeat(32);
const NODE = '33'.repeat(32);

const sample: GuildReceipt = {
	kind: 'keyspace.join',
	keyspaceId: 'aa'.repeat(32),
	contentHash: 'aa'.repeat(32),
	actor: 'bb'.repeat(32),
	logPath: '/tmp/guild-ops.log'
};

describe('GuildStore', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('copy says guild and the receipt kind is keyspace', () => {
		expect(GUILD_LABEL).toBe('Guild');
		expect(sample.kind.startsWith('keyspace.')).toBe(true);
		expect(sample.kind.includes('rename')).toBe(false);
		expect(sample.kind.startsWith('guild.')).toBe(false);
	});

	it('join needs only a seed; change and nest need a guild; assign needs a node', () => {
		const store = new GuildStore();
		expect(store.can('join')).toBe(false);
		store.actorSeed = SEED;
		expect(store.can('join')).toBe(true);
		expect(store.can('change')).toBe(false);
		expect(store.can('nest')).toBe(false);
		expect(store.can('assign-node')).toBe(false);
		store.guildId = GUILD;
		expect(store.can('change')).toBe(true);
		expect(store.can('nest')).toBe(true);
		expect(store.can('assign-node')).toBe(false);
		store.nodeId = NODE;
		expect(store.can('assign-node')).toBe(true);
	});

	it('does not invoke apply_network_name', async () => {
		vi.mocked(invoke).mockResolvedValue(sample as never);
		const store = new GuildStore();
		store.actorSeed = SEED;
		await store.sign('join');
		expect(invoke).toHaveBeenCalledTimes(1);
		expect(invoke).toHaveBeenCalledWith('sign_guild_op', {
			input: {
				verb: 'join',
				actorSeedHex: SEED,
				guildIdHex: undefined,
				nodeIdHex: undefined
			}
		});
		expect(vi.mocked(invoke).mock.calls.some((c) => c[0] === 'apply_network_name')).toBe(false);
		expect(store.receipt?.kind).toBe('keyspace.join');
	});

	it('empty seed does not invoke', async () => {
		const store = new GuildStore();
		await store.sign('join');
		expect(invoke).not.toHaveBeenCalled();
	});
});
