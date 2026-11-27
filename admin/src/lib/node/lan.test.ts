import { describe, expect, it } from 'vitest';
import type { Resource } from '$lib/discovery/resources';
import { lanForMesh, lanTarget } from './lan';

describe('lanTarget', () => {
	it('prefers the 10.42 gateway over another address on the same host', () => {
		expect(
			lanTarget([
				{ address: '192.168.1.20', ifindex: 2 },
				{ address: '10.42.61.1', ifindex: 2 },
				{ address: 'fe80::1%2', ifindex: 2 }
			])
		).toEqual({ address: '10.42.61.1', scopeId: 0 });
	});

	it('uses another gateway, then any other unicast address', () => {
		expect(lanTarget([{ address: '192.168.10.1', ifindex: 3 }])).toEqual({
			address: '192.168.10.1',
			scopeId: 0
		});
		expect(
			lanTarget([
				{ address: '239.255.255.250', ifindex: 1 },
				{ address: 'fe80::2', ifindex: 4 }
			])
		).toEqual({ address: 'fe80::2', scopeId: 4 });
	});

	it('skips multicast, broadcast, and unspecified addresses', () => {
		expect(
			lanTarget([
				{ address: '255.255.255.255', ifindex: 1 },
				{ address: '0.0.0.0', ifindex: 1 },
				{ address: '224.0.0.1', ifindex: 1 }
			])
		).toBeNull();
	});

	it('finds the gateway stored for a mesh node id', () => {
		const resources = [
			{
				meshNodeId: 'AA',
				rows: [{ address: '10.42.1.1', ifindex: 2 }]
			}
		] as Resource[];
		expect(lanForMesh(resources, 'aa')).toEqual({ address: '10.42.1.1', scopeId: 0 });
		expect(lanForMesh(resources, 'missing')).toBeNull();
	});
});
