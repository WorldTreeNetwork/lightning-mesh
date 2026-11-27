import { describe, expect, it } from 'vitest';
import { lightningDetail, publishedVersion } from './lightning-detail';

describe('lightningDetail', () => {
	it('shows the radio link, the strongest peer, and health', () => {
		const detail = lightningDetail({
			iface: 'wlp191s0',
			state: 'reachable',
			node: { node_id: 'aa', version: 1 },
			radio: {
				mesh_if: 'phy1-mesh0',
				channel: 36,
				stations: [
					{ mac: 'aa', signal_dbm: -40, expected_throughput_mbps: 1, inactive_ms: 2 },
					{ mac: 'bb', signal_dbm: -10, expected_throughput_mbps: 5, inactive_ms: 1 }
				],
				mpaths: []
			},
			health: { status: 'ok' },
			healthError: null
		});
		expect(detail.link).toBe('phy1-mesh0 · ch 36');
		expect(detail.strength).toBe('-10 dBm');
		expect(detail.version).toBe('—');
		expect(detail.status).toBe('reachable · ok');
	});

	it('uses a published version string and the scan interface when radio is missing', () => {
		expect(publishedVersion({ mesh_version: '0.4.2' })).toBe('0.4.2');
		const detail = lightningDetail({
			iface: 'wlp191s0',
			state: 'reachable',
			node: { mesh_version: '0.4.2' },
			radio: null,
			health: null,
			healthError: 'timed out'
		});
		expect(detail.link).toBe('wlp191s0');
		expect(detail.strength).toBe('—');
		expect(detail.version).toBe('0.4.2');
		expect(detail.status).toBe('reachable · no answer');
		expect(detail.statusTitle).toBe('timed out');
	});
});