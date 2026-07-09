// Seed <-> recovery-phrase conversion for exporting/importing a rung-1 identity
// (docs/network-coordination/user-identity.md — key portability). The 32-byte
// Ed25519 secret seed maps to a 24-word BIP39 mnemonic. @scure/bip39 is pure JS
// and does its work synchronously with no WebCrypto, so it runs in the insecure
// plain-HTTP `hello.mesh` context (only mnemonicToSeed*, which we don't use,
// needs subtle). No external hosts — the English wordlist is bundled.

import { entropyToMnemonic, mnemonicToEntropy, validateMnemonic } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';
import { bytesToHex, hexToBytes } from './hex';

/** 32-byte secret seed -> 24-word recovery phrase. */
export function seedToMnemonic(seed: Uint8Array): string {
	if (seed.byteLength !== 32) {
		throw new Error(`seed must be 32 bytes, got ${seed.byteLength}`);
	}
	return entropyToMnemonic(seed, wordlist);
}

/** 24-word recovery phrase -> 32-byte secret seed. Throws on an invalid phrase. */
export function mnemonicToSeed(mnemonic: string): Uint8Array {
	const normalized = normalizeMnemonic(mnemonic);
	if (!validateMnemonic(normalized, wordlist)) {
		throw new Error('Not a valid recovery phrase — check the words and their order.');
	}
	return mnemonicToEntropy(normalized, wordlist);
}

/** Collapse whitespace/casing so pasted phrases with odd spacing still validate. */
export function normalizeMnemonic(mnemonic: string): string {
	return mnemonic.trim().toLowerCase().split(/\s+/).join(' ');
}

/**
 * Accept either a 24-word recovery phrase or a raw 64-hex-char seed and return
 * the 32-byte secret seed. Used by the import flow, which takes one text box.
 */
export function parseSecretInput(input: string): Uint8Array {
	const trimmed = input.trim();
	if (/^[0-9a-fA-F]{64}$/.test(trimmed)) {
		return hexToBytes(trimmed.toLowerCase());
	}
	return mnemonicToSeed(trimmed);
}

export function seedToHex(seed: Uint8Array): string {
	return bytesToHex(seed);
}
