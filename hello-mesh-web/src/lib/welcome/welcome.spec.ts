import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import {
	initialInvitation,
	reduceInvitation,
	mayCreateIdentity,
	internetCopy,
	storageVerdict,
	hubUrls,
	regularBrowserPath,
	regularBrowserLimit,
	portalAsksForSeed,
	type Action
} from './welcome';

const run = (...actions: Action[]) => actions.reduce(reduceInvitation, initialInvitation);

describe('decline creates nothing', () => {
	it.each([
		[['explore']],
		[['dismiss']],
		[['justInternet']],
		[['startCreate', 'cancelConsent']],
		[['confirm']]
	] as [Action[]][])('%j never permits identity creation', (actions) => {
		expect(mayCreateIdentity(run(...actions))).toBe(false);
	});

	it('only consent then confirm permits creation', () => {
		expect(mayCreateIdentity(run('startCreate'))).toBe(false);
		expect(mayCreateIdentity(run('startCreate', 'confirm'))).toBe(true);
	});

	it('the welcome flow imports no routing, portal pass, or identity-write path', () => {
		const src = readFileSync(new URL('./welcome.ts', import.meta.url), 'utf8').replace(
			/\/\/.*|\/\*[\s\S]*?\*\//g,
			''
		);
		expect(src).not.toMatch(/fetch\(|XMLHttpRequest|capport|portal\/pass|firewall|saveIdentity/i);
	});
});

describe('no-upstream hub', () => {
	it('copy is correct in every route state and never claims a grant', () => {
		expect(internetCopy('offline')).toMatch(/local-only/);
		expect(internetCopy('online')).toMatch(/already available/);
		for (const u of ['online', 'offline', 'unknown'] as const) {
			expect(internetCopy(u)).not.toMatch(/restored|you are now online|granted/i);
		}
	});

	it('advertises name and LAN gateway with no token, even when nothing is published', () => {
		expect(hubUrls('10.42.7.1')).toEqual(['http://hello.mesh/', 'http://10.42.7.1/']);
		expect(hubUrls()).toEqual(['http://hello.mesh/']);
		for (const u of hubUrls('10.42.7.1')) expect(u).not.toMatch(/[?#]/);
	});
});

describe('portal key limit', () => {
	const ok = { userAgent: 'Firefox', hasIndexedDb: true, writeReadOk: true };

	it('fails toward unverified', () => {
		expect(storageVerdict(undefined)).toBe('unverified');
		expect(storageVerdict({ ...ok, writeReadOk: false })).toBe('unverified');
		expect(storageVerdict({ ...ok, userAgent: 'CaptiveNetworkSupport-355 wispr' })).toBe(
			'unverified'
		);
		expect(storageVerdict({ ...ok, portalHint: true })).toBe('unverified');
		expect(storageVerdict(ok)).toBe('verified');
	});

	it('offers a regular-browser path with no key in the URL and no seed request', () => {
		expect(regularBrowserLimit).toMatch(/may .*not keep keys/);
		for (const u of regularBrowserPath('10.42.7.1')) {
			expect(u).not.toMatch(/[?#]|secret|seed|key/i);
		}
		expect(portalAsksForSeed).toBe(false);
	});
});
