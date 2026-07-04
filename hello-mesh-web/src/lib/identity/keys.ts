// Rung-1 "soft custody" identity keypair: pure-JS Ed25519 (@noble/ed25519 v3),
// generated and held in the browser. Plain HTTP `hello.mesh` is an INSECURE
// context, so `crypto.subtle` (WebCrypto) is undefined — and noble's *async*
// hashing reaches for it (crypto.subtle.digest for SHA-512), which throws
// "crypto.subtle must be defined". The honest fix, per
// docs/network-coordination/user-identity.md §3/§4.6: install a pure-JS
// SHA-512 (@noble/hashes) as noble's hash hook and use the SYNC functions,
// which need only `crypto.getRandomValues` (available in insecure contexts).
// The key stays extractable by the serving node — never app/hardware custody.
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha2.js';
import { bytesToHex, hexToBytes } from './hex';

// Point noble at the pure-JS hash so it never touches crypto.subtle.
ed.hashes.sha512 = sha512;
ed.hashes.sha512Async = async (m: Uint8Array) => sha512(m);

export interface KeyPair {
	publicKey: Uint8Array;
	secretKey: Uint8Array;
}

export async function generateKeyPair(): Promise<KeyPair> {
	const { secretKey, publicKey } = ed.keygen();
	return { secretKey, publicKey };
}

/** Sign a hex-encoded challenge, returning the hex-encoded signature. */
export async function signChallengeHex(
	secretKey: Uint8Array,
	challengeHex: string
): Promise<string> {
	const signature = ed.sign(hexToBytes(challengeHex), secretKey);
	return bytesToHex(signature);
}

export function publicKeyHex(publicKey: Uint8Array): string {
	return bytesToHex(publicKey);
}
