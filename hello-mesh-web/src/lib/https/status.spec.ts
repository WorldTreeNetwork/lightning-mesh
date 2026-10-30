import { describe, expect, it } from 'vitest';
import type { DirectoryHttpsName } from '../directory/api';
import { httpsNameDisplay, httpsStatus } from './status';

const DAY = 86_400;
const NOW = 1_800_000_000;
const name: DirectoryHttpsName = {
	fqdn: 'n-example.house.mesh.worldtree.network',
	not_after: NOW + 10 * DAY,
	state: 'due'
};

describe('httpsStatus', () => {
	it('expires at and after not_after, even when renewal failed', () => {
		for (const failed of [false, true]) {
			expect(httpsStatus(NOW, NOW, failed)).toBe('expired');
			expect(httpsStatus(NOW + 1, NOW, failed)).toBe('expired');
		}
	});

	it('shows failure ahead of due or ok while the certificate is live', () => {
		for (const remaining of [1, 10 * DAY, 10 * DAY + 1]) {
			expect(httpsStatus(NOW, NOW + remaining, true)).toBe('failing');
		}
	});

	it('is due only for more than zero and at most ten days', () => {
		for (const remaining of [1, 10 * DAY - 1, 10 * DAY]) {
			expect(httpsStatus(NOW, NOW + remaining, false)).toBe('due');
		}
		expect(httpsStatus(NOW, NOW + 10 * DAY + 1, false)).toBe('ok');
	});
});

describe('httpsNameDisplay', () => {
	it('shows the exact ten-day offline validity line', () => {
		expect(httpsNameDisplay(name, NOW)).toEqual({
			state: 'due',
			url: `https://${name.fqdn}`,
			message: '10 days of HTTPS left offline'
		});
	});

	it('rounds partial days up without extending the due window', () => {
		expect(httpsNameDisplay({ ...name, not_after: NOW + 1 }, NOW).message).toBe(
			'1 days of HTTPS left offline'
		);
		expect(httpsNameDisplay({ ...name, not_after: NOW + 10 * DAY + 1 }, NOW)).toMatchObject({
			state: 'ok',
			message: '11 days of HTTPS left offline'
		});
	});

	it('keeps a live failing name openable with its remaining validity', () => {
		expect(httpsNameDisplay({ ...name, state: 'failing' }, NOW)).toMatchObject({
			state: 'failing',
			url: `https://${name.fqdn}`,
			message: '10 days of HTTPS left offline'
		});
	});

	it('removes the link and explains renewal when a stale record expires', () => {
		for (const state of ['ok', 'due', 'failing', 'expired'] as const) {
			for (const now of [name.not_after, name.not_after + DAY]) {
				const display = httpsNameDisplay({ ...name, state }, now);
				expect(display.state).toBe('expired');
				expect(display.url).toBeUndefined();
				expect(display.message).toBe('Connect a mesh gateway to the internet to renew HTTPS.');
			}
		}
	});

	it('never revives a published expired record when the visitor clock is behind', () => {
		expect(httpsNameDisplay({ ...name, state: 'expired' }, NOW).url).toBeUndefined();
	});

	it('fails closed when expiry metadata is invalid', () => {
		expect(httpsNameDisplay({ ...name, not_after: NaN }, NOW).url).toBeUndefined();
	});
});
