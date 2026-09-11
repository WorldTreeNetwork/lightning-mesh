import { describe, expect, it, vi } from 'vitest';
import * as ed from '@noble/ed25519';
import { generateKeyPair } from '$lib/identity/keys';
import { hexToBytes } from '$lib/identity/hex';
import type { Directory } from './api';
import type { TopoGraph } from '$lib/topology/graph';
import {
	COORDINATE_STAMP_DOMAIN,
	coordinateStampSigningBytes,
	degreesToE7,
	hasGps,
	metresToMm,
	shouldPostStamp,
	stampTargets,
	submitCoordinateStamp
} from './stamp';

const directory: Directory = {
	version: 1,
	node: {
		node_id: 'wr3000s-a',
		subnet: '10.42.243.0/24',
		backhaul_addr: '10.254.242.84',
		name: 'Front Porch',
		lat: 37.7749,
		lon: -122.4194,
		stamped_at: 1_700_000_000,
		stamper: 'phone-ada'
	},
	neighbors: [
		{
			node_id: 'm3000-b',
			addrs: ['10.254.12.214'],
			subnet: '10.42.12.0/24',
			name: 'Kitchen'
		},
		{
			node_id: 'tr3000',
			addrs: ['10.254.61.115'],
			subnet: '10.42.61.0/24',
			name: 'Workshop'
		}
	],
	identities: [],
	services: []
};

const graph: TopoGraph = {
	nodes: [
		{
			key: '10.254.242.84',
			label: '243',
			nodeId: 'wr3000s-a',
			name: 'Front Porch',
			subnet: '10.42.243.0/24',
			isSelf: true,
			radio: {
				version: 1,
				backhaul_addr: '10.254.242.84',
				mesh_if: 'phy1-mesh0',
				mesh_mac: 'aa:bb:cc:d9:85:af',
				channel: 36,
				freq_mhz: 5180,
				collected_at_unix: 1,
				stations: [
					{
						mac: 'aa:bb:cc:e7:ba:9d',
						signal_dbm: -42,
						expected_throughput_mbps: 900,
						inactive_ms: 0
					},
					{
						mac: 'aa:bb:cc:98:fb:10',
						signal_dbm: -78,
						expected_throughput_mbps: 80,
						inactive_ms: 0
					}
				],
				mpaths: []
			}
		},
		{
			key: '10.254.12.214',
			label: '12',
			nodeId: 'm3000-b',
			name: 'Kitchen',
			subnet: '10.42.12.0/24',
			isSelf: false,
			radio: {
				version: 1,
				backhaul_addr: '10.254.12.214',
				mesh_if: 'phy1-mesh0',
				mesh_mac: 'aa:bb:cc:e7:ba:9d',
				channel: 36,
				freq_mhz: 5180,
				collected_at_unix: 1,
				stations: [],
				mpaths: []
			}
		},
		{
			key: '10.254.61.115',
			label: '61',
			nodeId: 'tr3000',
			name: 'Workshop',
			subnet: '10.42.61.0/24',
			isSelf: false,
			radio: {
				version: 1,
				backhaul_addr: '10.254.61.115',
				mesh_if: 'phy1-mesh0',
				mesh_mac: 'aa:bb:cc:98:fb:10',
				channel: 36,
				freq_mhz: 5180,
				collected_at_unix: 1,
				stations: [],
				mpaths: []
			}
		}
	],
	edges: []
};

describe('stampTargets', () => {
	it('is empty when the directory has no self node', () => {
		expect(
			stampTargets({
				version: 1,
				node: null,
				neighbors: directory.neighbors,
				identities: [],
				services: []
			})
		).toEqual([]);
	});

	it('defaults to the associated node first, then ranks neighbors by signal', () => {
		const targets = stampTargets(directory, graph);
		expect(targets.map((t) => t.nodeId)).toEqual(['wr3000s-a', 'm3000-b', 'tr3000']);
		expect(targets[0]?.isSelf).toBe(true);
		expect(targets[1]?.signalDbm).toBe(-42);
		expect(targets[2]?.signalDbm).toBe(-78);
		expect(targets[0]?.lat).toBe(37.7749);
		expect(targets[0]?.lon).toBe(-122.4194);
	});
});

describe('shouldPostStamp', () => {
	const targets = stampTargets(directory, graph);

	it('pick + GPS writes', () => {
		expect(
			shouldPostStamp({
				intent: 'confirm',
				targets,
				selectedNodeId: 'wr3000s-a',
				lat: 37.7749,
				lon: -122.4194
			})
		).toBe(true);
	});

	it('cancel writes nothing', () => {
		expect(
			shouldPostStamp({
				intent: 'cancel',
				targets,
				selectedNodeId: 'wr3000s-a',
				lat: 37.7749,
				lon: -122.4194
			})
		).toBe(false);
	});

	it('no-GPS writes nothing', () => {
		expect(
			shouldPostStamp({
				intent: 'confirm',
				targets,
				selectedNodeId: 'wr3000s-a',
				lat: null,
				lon: null
			})
		).toBe(false);
		expect(hasGps(undefined, undefined)).toBe(false);
		expect(hasGps('', '')).toBe(false);
	});

	it('empty list writes nothing', () => {
		expect(
			shouldPostStamp({
				intent: 'confirm',
				targets: [],
				selectedNodeId: 'wr3000s-a',
				lat: 1,
				lon: 2
			})
		).toBe(false);
	});
});

describe('e7 conversion', () => {
	it('matches the CRDT microdegree unit', () => {
		expect(degreesToE7(37.0)).toBe(370_000_000);
		expect(degreesToE7(-122.0)).toBe(-1_220_000_000);
		expect(metresToMm(1.5)).toBe(1_500);
	});
});

describe('submitCoordinateStamp', () => {
	it('POSTs a signed claim after fetching a challenge', async () => {
		const { publicKey, secretKey } = await generateKeyPair();
		const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
			const url = String(input);
			if (url === '/api/challenge') {
				return {
					ok: true,
					json: async () => ({ challenge: 'aa'.repeat(32) })
				} as Response;
			}
			if (url === '/api/coordinate-stamp') {
				const body = JSON.parse(String(init?.body)) as {
					sig: string;
					challenge: string;
					node_id: string;
					lat: number;
					lon: number;
					stamped_at: number;
				};
				expect(body.node_id).toBe('wr3000s-a');
				expect(body.lat).toBe(37);
				expect(body.lon).toBe(-122);
				const msg = coordinateStampSigningBytes({
					challenge: body.challenge,
					nodeId: body.node_id,
					lat: body.lat,
					lon: body.lon,
					stampedAt: body.stamped_at
				});
				expect(new TextDecoder().decode(msg).startsWith(COORDINATE_STAMP_DOMAIN)).toBe(true);
				const ok = await ed.verifyAsync(hexToBytes(body.sig), msg, publicKey);
				expect(ok).toBe(true);
				return { ok: true, text: async () => '' } as Response;
			}
			throw new Error(`unexpected fetch ${url}`);
		});

		await submitCoordinateStamp(
			{
				secretKey,
				publicKey,
				nodeId: 'wr3000s-a',
				lat: 37,
				lon: -122,
				stampedAt: 1_700_000_042
			},
			fetchImpl as unknown as typeof fetch
		);
		expect(fetchImpl).toHaveBeenCalledTimes(2);
	});
});
