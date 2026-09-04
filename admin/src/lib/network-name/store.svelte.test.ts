import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

import { invoke } from '@tauri-apps/api/core';
import { NETWORK_NAME_LABEL, NetworkNameStore } from './store.svelte';
import type { ApplyReport } from './api';

function mockApply(value: ApplyReport) {
	vi.mocked(invoke).mockImplementation((cmd: string) => {
		if (cmd === 'apply_network_name') return Promise.resolve(value as never);
		return Promise.resolve(undefined as never);
	});
}

const sample: ApplyReport = {
	updated: ['m3000'],
	skipped: ['ap3000'],
	halted: null,
	ok: true,
	log: '===== m3000 (GL-MT3000) — root@10.254.3.4 =====\n>> m3000: OK\n'
};

describe('NetworkNameStore', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('copy says network name and does not say guild', () => {
		expect(NETWORK_NAME_LABEL).toBe('Network name');
		expect(NETWORK_NAME_LABEL.toLowerCase()).toContain('network name');
		expect(NETWORK_NAME_LABEL.toLowerCase()).not.toContain('guild');
	});

	it('empty name does not invoke', async () => {
		const store = new NetworkNameStore();
		store.name = '';
		await store.apply();
		store.name = '   ';
		await store.apply();
		expect(invoke).not.toHaveBeenCalled();
		expect(store.applying).toBe(false);
		expect(store.report).toBeNull();
	});

	it('canApply is false when empty or applying', () => {
		const store = new NetworkNameStore();
		expect(store.canApply).toBe(false);
		store.name = 'Lightning Mesh';
		expect(store.canApply).toBe(true);
		store.applying = true;
		expect(store.canApply).toBe(false);
	});

	it('invokes apply_network_name with the trimmed name', async () => {
		const store = new NetworkNameStore();
		mockApply(sample);
		store.name = '  Lightning Mesh  ';
		await store.apply();
		expect(invoke).toHaveBeenCalledWith('apply_network_name', { name: 'Lightning Mesh' });
		expect(store.report?.updated).toEqual(['m3000']);
		expect(store.report?.skipped).toEqual(['ap3000']);
		expect(store.applying).toBe(false);
		expect(store.error).toBeNull();
	});

	it('surfaces invoke failures without throwing', async () => {
		vi.mocked(invoke).mockRejectedValue(new Error('apply-network-name.sh missing'));
		const store = new NetworkNameStore();
		store.name = 'Lightning Mesh';
		await store.apply();
		expect(store.error).toBe('apply-network-name.sh missing');
		expect(store.applying).toBe(false);
		expect(store.report).toBeNull();
	});
});
