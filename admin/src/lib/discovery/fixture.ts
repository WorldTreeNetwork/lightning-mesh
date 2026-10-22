import type { ScanResult } from './api';

/** Browser-only preview (`?demo=1`) so the list can be exercised without Tauri. */
export function demoScan(): ScanResult {
	return {
		interfaces: [
			{
				name: 'wlp191s0',
				index: 2,
				mac: '8e:d6:da:29:d9:8d',
				is_up: true,
				is_loopback: false,
				link_local: ['fe80::1%wlp191s0'],
				unique_local: ['fd01:d28c:7e4a::1', 'fd01:d28c:7e4a::65b']
			},
			{
				name: 'eth0',
				index: 3,
				mac: 'aa:bb:cc:dd:ee:ff',
				is_up: true,
				is_loopback: false,
				link_local: ['fe80::1%eth0'],
				unique_local: []
			},
			{
				name: 'tailscale0',
				index: 4,
				mac: null,
				is_up: true,
				is_loopback: false,
				link_local: [],
				unique_local: ['fd7a:115c:a1e0::1']
			},
			{
				name: 'wg0',
				index: 5,
				mac: null,
				is_up: true,
				is_loopback: false,
				link_local: [],
				unique_local: []
			},
			{
				name: 'br-4fbd877829e3',
				index: 6,
				mac: '02:42:ac:11:00:01',
				is_up: true,
				is_loopback: false,
				link_local: ['fe80::42:acff:fe11:1%br-4fbd877829e3'],
				unique_local: []
			}
		],
		neighbors: [
			{
				iface: 'wlp191s0',
				ifindex: 2,
				address: 'fd01:d28c:7e4a::1',
				scoped: 'fd01:d28c:7e4a::1',
				base58: 'YF4RhMGBc3LA1xkSVuXzpg',
				scope: 'unique-local',
				mac: '8e:d6:da:29:d9:8d',
				state: 'local',
				kind: 'local',
				source: 'addr'
			},
			{
				iface: 'wlp191s0',
				ifindex: 2,
				address: 'fd01:d28c:7e4a::65b',
				scoped: 'fd01:d28c:7e4a::65b',
				base58: 'YF4RhMGBc3LA1xkSVuY1Ji',
				scope: 'unique-local',
				mac: '8e:d6:da:29:d9:8d',
				state: 'local',
				kind: 'local',
				source: 'addr'
			},
			{
				iface: 'eth0',
				ifindex: 3,
				address: 'fe80::1',
				scoped: 'fe80::1%eth0',
				base58: 'YRka4zYGRkixTpb4LjCkzL',
				scope: 'link-local',
				mac: 'aa:bb:cc:dd:ee:ff',
				state: 'local',
				kind: 'local',
				source: 'addr'
			},
			{
				iface: 'eth0',
				ifindex: 3,
				address: 'fe80::abcd:1',
				scoped: 'fe80::abcd:1%eth0',
				base58: 'YRka4zYGRkixTpb4LjNear',
				scope: 'link-local',
				mac: 'b2:10:00:00:00:21',
				state: 'stale',
				kind: 'neighbor',
				source: 'probe'
			},
			{
				iface: 'eth0',
				ifindex: 3,
				address: 'fd01:d28c:7e4a::21',
				scoped: 'fd01:d28c:7e4a::21',
				base58: 'YNearbyUlaExample00021',
				scope: 'unique-local',
				mac: 'b2:10:00:00:00:21',
				state: 'reachable',
				kind: 'neighbor',
				source: 'neigh'
			},
			{
				iface: 'tailscale0',
				ifindex: 4,
				address: 'fd7a:115c:a1e0::1',
				scoped: 'fd7a:115c:a1e0::1',
				base58: 'YTailscaleExample0001',
				scope: 'unique-local',
				mac: null,
				state: 'local',
				kind: 'local',
				source: 'addr'
			},
			{
				iface: 'br-4fbd877829e3',
				ifindex: 6,
				address: 'fe80::42:acff:fe11:1',
				scoped: 'fe80::42:acff:fe11:1%br-4fbd877829e3',
				base58: 'YBridgeExample00000001',
				scope: 'link-local',
				mac: '02:42:ac:11:00:01',
				state: 'local',
				kind: 'local',
				source: 'addr'
			}
		],
		probed: true,
		probe_error: null
	};
}
