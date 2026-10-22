import { describe, expect, test } from 'vitest';
import type { LinkLocalNeighbor } from './api';
import { groupResources } from './resources';

function row(partial: Partial<LinkLocalNeighbor> & Pick<LinkLocalNeighbor, 'address' | 'iface' | 'ifindex'>): LinkLocalNeighbor {
	return {
		scoped: partial.address,
		base58: partial.address,
		scope: partial.scope ?? 'link-local',
		mac: partial.mac ?? null,
		state: partial.state ?? 'reachable',
		kind: partial.kind ?? 'neighbor',
		source: 'probe',
		router: partial.router ?? false,
		hostname: partial.hostname ?? null,
		...partial
	};
}

describe('groupResources', () => {
	test('folds a ULA and a link-local that share a MAC on one interface', () => {
		const resources = groupResources([
			row({ iface: 'eth0', ifindex: 2, address: 'fe80::1', mac: 'aa:bb', state: 'stale' }),
			row({
				iface: 'eth0',
				ifindex: 2,
				address: 'fd01::1',
				scope: 'unique-local',
				mac: 'AA:BB',
				state: 'reachable'
			})
		]);
		expect(resources).toHaveLength(1);
		expect(resources[0].guess).toBe(true);
		expect(resources[0].primary.address).toBe('fd01::1');
		expect(resources[0].state).toBe('reachable');
		expect(resources[0].rows).toHaveLength(2);
	});

	test('joins the same MAC across interfaces and joins different MACs by endpoint id', () => {
		const sameNic = groupResources([
			row({ iface: 'eth0', ifindex: 2, address: 'fe80::1', mac: 'aa:bb' }),
			row({ iface: 'wlan0', ifindex: 3, address: '10.42.61.119', scope: 'ipv4', mac: 'aa:bb' })
		]);
		expect(sameNic).toHaveLength(1);
		expect(sameNic[0].primary.address).toBe('10.42.61.119');

		const joined = groupResources([
			row({ iface: 'eth0', ifindex: 2, address: 'fe80::1', mac: 'aa:bb', iroh: 'endpoint-a' }),
			row({ iface: 'wlan0', ifindex: 3, address: 'fd01::2', scope: 'unique-local', mac: 'cc:dd', iroh: 'endpoint-a' })
		]);
		expect(joined).toHaveLength(1);
		expect(joined[0].guess).toBe(false);
	});

	test('joins an IPv4 lease and an IPv6 address that share an mDNS name, and names the gateway', () => {
		const resources = groupResources([
			row({
				iface: 'eth0',
				ifindex: 2,
				address: '10.42.61.1',
				scope: 'ipv4',
				mac: '80:af:ca:98:fb:0f',
				router: true
			}),
			row({
				iface: 'eth0',
				ifindex: 2,
				address: '10.42.61.119',
				scope: 'ipv4',
				mac: '98:fe:54:08:25:fc',
				hostname: 'edgebox.local'
			}),
			row({
				iface: 'eth0',
				ifindex: 2,
				address: 'fd5b:4dda:dc1e::b7d',
				scope: 'unique-local',
				mac: '98:fe:54:08:25:fc',
				hostname: 'edgebox.local'
			})
		]);
		const edge = resources.find((resource) => resource.hostname === 'edgebox.local');
		expect(edge?.rows).toHaveLength(2);
		expect(edge?.via).toEqual(['Gateway']);
		expect(resources.filter((resource) => resource.router)).toHaveLength(1);
	});

	test('names a Lightning node from the hello identity and prefers that over gateway', () => {
		const resources = groupResources([
			row({
				iface: 'eth0',
				ifindex: 2,
				address: '10.42.61.1',
				scope: 'ipv4',
				mac: '80:af:ca:98:fb:0f',
				router: true,
				mesh_name: 'tr3000',
				mesh_node_id: 'node-tr',
				mesh_subnet: '10.42.61.0/24',
				mesh_backhaul: '10.254.61.115'
			}),
			row({
				iface: 'eth0',
				ifindex: 2,
				address: 'fe80::82af:caff:fe98:fb0f',
				mac: '80:af:ca:98:fb:0f',
				router: true
			})
		]);
		expect(resources).toHaveLength(1);
		expect(resources[0].meshName).toBe('tr3000');
		expect(resources[0].meshSubnet).toBe('10.42.61.0/24');
	});

	test('does not merge addresses that have no MAC and no endpoint id', () => {
		const resources = groupResources([
			row({ iface: 'eth0', ifindex: 2, address: 'fe80::1' }),
			row({ iface: 'eth0', ifindex: 2, address: 'fe80::2' })
		]);
		expect(resources).toHaveLength(2);
		expect(resources.every((r) => r.guess === false)).toBe(true);
	});
});
