import { describe, expect, it } from 'vitest';
import { guildSignInput } from './sign-input';

const GUILD = '22'.repeat(32);

describe('guildSignInput', () => {
	it('starts a house without a guild id', () => {
		expect(guildSignInput('join', GUILD)).toEqual({
			verb: 'join',
			actorSeedHex: '',
			guildIdHex: undefined,
			nodeIdHex: undefined
		});
	});

	it('nests and changes the selected guild and does not carry a network name', () => {
		for (const verb of ['nest', 'change'] as const) {
			const input = guildSignInput(verb, GUILD);
			expect(input.verb).toBe(verb);
			expect(input.guildIdHex).toBe(GUILD);
			expect(input.nodeIdHex).toBeUndefined();
			expect(JSON.stringify(input)).not.toContain('ssid');
			expect(JSON.stringify(input)).not.toContain('network');
		}
	});
});
