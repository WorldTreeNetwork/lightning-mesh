import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

import { invoke } from '@tauri-apps/api/core';
import { DiscoveryStore } from './store.svelte';
import type { ScanResult } from './api';

function mockScan(value: ScanResult) {
	vi.mocked(invoke).mockImplementation((cmd: string) => {
		if (cmd === 'scan_link_local') return Promise.resolve(value as never);
		return Promise.resolve(undefined as never);
	});
}

const sample: ScanResult = {
	interfaces: [
		{
			name: 'eth0',
			index: 2,
			mac: 'aa:bb:cc:dd:ee:ff',
			is_up: true,
			is_loopback: false,
			link_local: ['fe80::1%eth0']
		},
		{
			name: 'wlan0',
			index: 3,
			mac: '11:22:33:44:55:66',
			is_up: true,
			is_loopback: false,
			link_local: ['fe80::2%wlan0']
		},
		{
			name: 'lo',
			index: 1,
			mac: null,
			is_up: true,
			is_loopback: true,
			link_local: []
		}
	],
	neighbors: [
		{
			iface: 'eth0',
			ifindex: 2,
			address: 'fe80::1',
			scoped: 'fe80::1%eth0',
			mac: 'aa:bb:cc:dd:ee:ff',
			state: 'local',
			kind: 'local',
			source: 'addr'
		},
		{
			iface: 'eth0',
			ifindex: 2,
			address: 'fe80::9',
			scoped: 'fe80::9%eth0',
			mac: '00:11:22:33:44:55',
			state: 'reachable',
			kind: 'neighbor',
			source: 'probe'
		},
		{
			iface: 'wlan0',
			ifindex: 3,
			address: 'fe80::2',
			scoped: 'fe80::2%wlan0',
			mac: '11:22:33:44:55:66',
			state: 'local',
			kind: 'local',
			source: 'addr'
		}
	],
	probed: true,
	probe_error: null
};

describe('DiscoveryStore', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('starts empty', () => {
		const store = new DiscoveryStore();
		expect(store.neighbors).toEqual([]);
		expect(store.selectedIface).toBe('all');
		expect(store.loading).toBe(false);
	});

	it('loads a scan over invoke and hides loopback from the rail', async () => {
		const store = new DiscoveryStore();
		mockScan(sample);
		await store.scan();
		expect(invoke).toHaveBeenCalledWith('scan_link_local');
		expect(store.interfaces).toHaveLength(3);
		expect(store.networkInterfaces.map((i) => i.name)).toEqual(['eth0', 'wlan0']);
		expect(store.neighbors).toHaveLength(3);
		expect(store.probed).toBe(true);
		expect(store.lastScan).toBeTypeOf('number');
	});

	it('filters neighbors by selected interface', async () => {
		const store = new DiscoveryStore();
		mockScan(sample);
		await store.scan();
		store.selectIface('eth0');
		expect(store.visibleNeighbors).toHaveLength(2);
		expect(store.neighborCount('wlan0')).toBe(1);
	});

	it('surfaces invoke failures without throwing', async () => {
		vi.mocked(invoke).mockRejectedValue(new Error('not in tauri'));
		const store = new DiscoveryStore();
		await store.scan();
		expect(store.error).toBe('not in tauri');
		expect(store.loading).toBe(false);
		expect(store.neighbors).toEqual([]);
	});
});
