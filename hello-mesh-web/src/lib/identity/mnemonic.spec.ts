import { describe, expect, it } from 'vitest';
import * as ed from '@noble/ed25519';
import { seedToMnemonic, mnemonicToSeed, parseSecretInput, normalizeMnemonic } from './mnemonic';
import { keyPairFromSecret } from './keys';
import { bytesToHex } from './hex';

function seedOf(fill: number): Uint8Array {
	return new Uint8Array(32).fill(fill);
}

describe('seed <-> mnemonic', () => {
	it('round-trips a 32-byte seed through a 24-word phrase', () => {
		const seed = ed.utils.randomSecretKey();
		const mnemonic = seedToMnemonic(seed);
		expect(mnemonic.split(' ')).toHaveLength(24);
		expect(bytesToHex(mnemonicToSeed(mnemonic))).toBe(bytesToHex(seed));
	});

	it('rejects a non-32-byte seed', () => {
		expect(() => seedToMnemonic(new Uint8Array(16))).toThrow();
	});

	it('rejects an invalid phrase', () => {
		expect(() =>
			mnemonicToSeed('not actually a valid bip39 recovery phrase at all nope')
		).toThrow();
	});

	it('normalizes casing and spacing before validating', () => {
		const mnemonic = seedToMnemonic(seedOf(7));
		const messy = `  ${mnemonic.toUpperCase().split(' ').join('   ')}  `;
		expect(bytesToHex(mnemonicToSeed(messy))).toBe(bytesToHex(seedOf(7)));
		expect(normalizeMnemonic(messy)).toBe(mnemonic);
	});
});

describe('parseSecretInput', () => {
	it('accepts a raw 64-hex seed', () => {
		const seed = seedOf(0xab);
		expect(bytesToHex(parseSecretInput(bytesToHex(seed)))).toBe(bytesToHex(seed));
	});

	it('accepts a recovery phrase', () => {
		const seed = ed.utils.randomSecretKey();
		expect(bytesToHex(parseSecretInput(seedToMnemonic(seed)))).toBe(bytesToHex(seed));
	});
});

describe('import round-trips the exact keypair', () => {
	it('derives the same public key from an exported-then-imported seed', () => {
		const original = ed.keygen();
		// Export path: secretKey (the seed) -> mnemonic.
		const mnemonic = seedToMnemonic(original.secretKey);
		// Import path: mnemonic -> seed -> keypair.
		const seed = mnemonicToSeed(mnemonic);
		const restored = keyPairFromSecret(seed);
		expect(bytesToHex(restored.secretKey)).toBe(bytesToHex(original.secretKey));
		expect(bytesToHex(restored.publicKey)).toBe(bytesToHex(original.publicKey));
	});
});
