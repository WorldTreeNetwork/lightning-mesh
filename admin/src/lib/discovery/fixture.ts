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
			}
		],
		probed: true,
		probe_error: null
	};
}
