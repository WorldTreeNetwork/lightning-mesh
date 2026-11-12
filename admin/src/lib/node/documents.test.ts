import { describe, expect, it } from 'vitest';
import { asDirectoryDocument, mergeServices } from './documents';
import { layoutRadio } from './radio-graph';

describe('directory services', () => {
	it('keeps one row per name, address, and port and names every reporter', () => {
		const rows = mergeServices([
			{
				node: 'ap3000',
				services: asDirectoryDocument({
					services: [{ name: 'hello', ip: '10.42.1.1', port: 80, protocol: 'tcp', txt: { path: '/' } }]
				}).services
			},
			{
				node: 'tr3000',
				services: [{ name: 'hello', ip: '10.42.1.1', port: 80, protocol: 'tcp', txt: {} }]
			}
		]);
		expect(rows).toHaveLength(1);
		expect(rows[0]?.reportedBy).toEqual(['ap3000', 'tr3000']);
	});

	it('is empty when every directory has an empty services array', () => {
		expect(mergeServices([{ node: 'ap3000', services: asDirectoryDocument({ services: [] }).services }])).toEqual(
			[]
		);
	});
});

describe('layoutRadio', () => {
	it('draws a station edge and keeps the path as a caption when that edge exists', () => {
		const graph = layoutRadio([
			{
				id: 'a',
				name: 'ap',
				quiet: false,
				mac: 'aa:aa:aa:aa:aa:01',
				stations: [
					{ mac: 'aa:aa:aa:aa:aa:02', signal_dbm: -40, expected_throughput_mbps: 12, inactive_ms: 4 }
				],
				mpaths: []
			},
			{
				id: 'b',
				name: 'tr',
				quiet: false,
				mac: 'AA:AA:AA:AA:AA:02',
				stations: [],
				mpaths: [{ dst: 'aa:aa:aa:aa:aa:01', next_hop: 'aa:aa:aa:aa:aa:01', metric: 9 }]
			},
			{ id: 'c', name: 'quiet', quiet: true }
		]);
		expect(graph.nodes.map((node) => node.id)).toEqual(['a', 'b', 'c']);
		expect(graph.edges.filter((edge) => !edge.quiet)).toEqual([
			{ from: 'a', to: 'b', label: '-40 dBm', quiet: false }
		]);
		expect(graph.edges.filter((edge) => edge.quiet)).toEqual([]);
		expect(graph.paths[0]).toContain('metric 9');
		expect(new Set(graph.nodes.map((node) => `${node.x},${node.y}`)).size).toBe(3);
	});

	it('uses a quieter path edge only when the station table has no matching edge', () => {
		const graph = layoutRadio([
			{
				id: 'a',
				name: 'ap',
				quiet: false,
				mac: '11:11:11:11:11:11',
				stations: [],
				mpaths: [{ dst: '22:22:22:22:22:22', next_hop: '22:22:22:22:22:22', metric: 3 }]
			},
			{ id: 'b', name: 'tr', quiet: false, mac: '22:22:22:22:22:22', stations: [], mpaths: [] }
		]);
		expect(graph.edges).toEqual([{ from: 'a', to: 'b', label: 'path', quiet: true }]);
	});
});
