import { describe, expect, it } from 'vitest';
import { hexToBytes } from '../identity/hex';
import fixtures from './fixtures/labels.json';
import { httpsLabel } from './label';

const HTTPS_LABEL = /^[a-z2-7]{16}$/;

describe('httpsLabel', () => {
	it('matches the shared Rust and TypeScript vectors', () => {
		for (const fixture of fixtures) {
			const label = httpsLabel(hexToBytes(fixture.owner_pubkey_hex));
			expect(label, fixture.name).toBe(fixture.label);
			expect(label, fixture.name).toHaveLength(16);
			expect(label, fixture.name).toMatch(HTTPS_LABEL);
		}
	});
});
