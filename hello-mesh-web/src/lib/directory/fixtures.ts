// Mock directory for previewing the whole page without a live mesh. Enabled via
// `?mockDirectory=1` (dev/demo only) — the directory store loads this instead of
// polling /api/directory, so People, Routers, and Services all render with
// believable data. Kept in sync with the topology fixture's node names so the
// two panels tell the same story. Recency spans every presence tier; timestamps
// are computed relative to load time so the demo always looks live.

import type { Directory } from './api';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

export function mockDirectory(now: number = Date.now()): Directory {
	return {
		version: 1,
		node: {
			node_id: 'wr3000s-a9f3e21c',
			subnet: '10.42.243.0/24',
			backhaul_addr: '10.254.242.84',
			name: 'Front Porch'
		},
		neighbors: [
			{
				node_id: 'm3000-b7c11d40',
				addrs: ['10.254.12.214'],
				subnet: '10.42.12.0/24',
				name: 'Kitchen'
			},
			{
				node_id: 'tr3000-5e0a99b2',
				addrs: ['10.254.61.115'],
				subnet: '10.42.61.0/24',
				name: 'Workshop'
			},
			{
				// Unnamed neighbor — exercises the muted subnet/hex fallback.
				node_id: 'm3000-2f8ab310',
				addrs: ['10.254.242.172'],
				subnet: '10.42.242.0/24'
			}
		],
		identities: [
			{
				username: 'a1b2c3d4e5f60718293a4b5c6d7e8f90112233445566778899aabbccddeeff00',
				display_name: 'Maya',
				last_seen_unix: now - 30_000 // here now
			},
			{
				username: 'f00dcafe0011223344556677889900aabbccddeeff00112233445566778899aa',
				display_name: 'Diego',
				last_seen_unix: now - 12 * MIN // seen 12m ago
			},
			{
				username: 'deadbeef99887766554433221100ffeeddccbbaa00112233445566778899aabb',
				display_name: 'Priya',
				last_seen_unix: now - 3 * HOUR // seen 3h ago
			},
			{
				username: '0011223344556677889900aabbccddeeff00112233445566778899aabbccddee',
				display_name: 'Sam',
				last_seen_unix: now - 2 * DAY // dormant — collapses into the group
			},
			{
				// No last_seen — older daemon / unknown recency, stays visible.
				username: 'abcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcab',
				display_name: 'Wanderer'
			}
		],
		services: [
			{
				name: 'Library._http._tcp',
				ip: '10.42.12.5',
				port: 8080,
				protocol: 'http',
				hostname: 'kitchen-pi.local'
			},
			{
				name: 'Village Wiki._https._tcp',
				ip: '10.42.61.9',
				port: 443,
				protocol: 'https',
				hostname: 'workshop.local'
			},
			{
				name: 'moq-relay._quic._udp',
				ip: '10.42.243.1',
				port: 4433,
				protocol: 'quic',
				host_mac: 'de:ad:be:ef:00:01',
				txt: { role: 'relay', v: '1' }
			}
		]
	};
}
