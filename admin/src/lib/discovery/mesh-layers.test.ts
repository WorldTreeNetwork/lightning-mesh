import { describe, expect, it } from 'vitest';
import { formatHops, gatewayOnInterface, hopDistances, macFromIpv6 } from './mesh-layers';

describe('mesh layers', () => {
	it('recovers a mesh MAC from the link-local address hello publishes', () => {
		expect(macFromIpv6('fe80::d40d:abff:fe5b:f965')).toBe('d6:0d:ab:5b:f9:65');
		expect(macFromIpv6('fe80::80af:caff:fee7:ba9d')).toBe('82:af:ca:e7:ba:9d');
		expect(macFromIpv6('10.42.203.1')).toBeNull();
	});

	it('counts hops outward from the attached router', () => {
		const hops = hopDistances(
			'd6:0d:ab:5b:f9:65',
			[
				{ dst: '82:af:ca:e7:ba:9d', nextHop: '82:af:ca:d9:85:af' },
				{ dst: '82:af:ca:d9:85:af', nextHop: '82:af:ca:d9:85:af' }
			],
			['82:af:ca:d9:85:af']
		);
		expect(hops.get('d6:0d:ab:5b:f9:65')).toBe(0);
		expect(hops.get('82:af:ca:d9:85:af')).toBe(1);
		expect(hops.get('82:af:ca:e7:ba:9d')).toBe(2);
		expect(formatHops(0)).toBe('here');
		expect(formatHops(1)).toBe('1 hop');
		expect(formatHops(2)).toBe('2 hops');
	});

	it('picks the lightning gateway on this computer’s subnet', () => {
		expect(
			gatewayOnInterface(
				['10.42.203.108/24', '100.121.90.25/32'],
				[
					{ id: 'leaf', addresses: ['10.42.12.1'] },
					{ id: 'ap', addresses: ['10.42.203.1', 'fe80::1'] }
				]
			)
		).toBe('ap');
	});
});
