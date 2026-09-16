import { blake3 } from '@noble/hashes/blake3.js';

const BASE32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';
const HTTPS_LABEL_BYTES = 10;

/** Derive the 16-character HTTPS DNS label for an Ed25519 owner public key. */
export function httpsLabel(ownerPublicKey: Uint8Array): string {
	const digestPrefix = blake3(ownerPublicKey).subarray(0, HTTPS_LABEL_BYTES);
	let label = '';
	let accumulator = 0;
	let bits = 0;

	for (const byte of digestPrefix) {
		accumulator = (accumulator << 8) | byte;
		bits += 8;

		while (bits >= 5) {
			bits -= 5;
			label += BASE32_ALPHABET[(accumulator >>> bits) & 0x1f];
			accumulator &= bits === 0 ? 0 : (1 << bits) - 1;
		}
	}

	return label;
}
