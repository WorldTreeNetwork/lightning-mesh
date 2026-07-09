// mjolnir-identity-assert v1 — the hello.mesh identity-provider handoff.
//
// A relying party (`wiki.mesh`, `chat.mesh`, …) redirects the browser to
// hello.mesh/assert; the user consents; hello.mesh signs a self-certifying
// assertion with the browser-held rung-1 key and redirects back with the token
// carried in the URL FRAGMENT (never the query — fragments are not sent to any
// server, so the token never leaks into node logs). The RP verifies the
// assertion OFFLINE: the pubkey is inside the payload, so no call back to
// hello.mesh is needed. See docs/network-coordination/identity-assertion.md.
//
// Pure functions only — no DOM, no IndexedDB — so they unit-test cleanly and
// an RP author can copy `verifyAssertion` verbatim.
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha2.js';
import { bytesToHex, hexToBytes } from './hex';

// Insecure-context safety: point noble at pure-JS SHA-512 (same as keys.ts) so
// it never reaches for crypto.subtle, which is undefined over plain HTTP.
ed.hashes.sha512 = sha512;
ed.hashes.sha512Async = async (m: Uint8Array) => sha512(m);

/** Domain-separation prefix. Mirrors name_claim_signing_message in
 *  crates/mjolnir-hello/src/routes.rs — the signed bytes are this prefix
 *  followed by a newline and the exact payload JSON. */
export const ASSERT_DOMAIN = 'mjolnir-identity-assert:v1';

/** How long an assertion is valid, in seconds. */
export const ASSERT_TTL_SECS = 300;

export type AssertPrompt = 'consent' | 'none';

export interface AssertRequest {
	audience: string;
	returnTo: string;
	nonce: string;
	prompt: AssertPrompt;
}

export type AssertErrorCode = 'access_denied' | 'interaction_required' | 'invalid_request';

export interface AssertRequestError {
	error: 'invalid_request';
}

/** The signed claim. Key order is load-bearing: signer and verifier both
 *  serialize this exact object-literal shape (see buildAssertionPayload). */
export interface AssertionPayload {
	v: 1;
	pubkey: string; // 64-hex ed25519 public key (self-certifying)
	display_name: string;
	audience: string; // RP origin
	nonce: string; // hex, echoed from the request
	issued_at: number; // unix secs
	expires_at: number; // issued_at + ASSERT_TTL_SECS
}

export interface AssertionToken {
	payload: string; // the exact JSON bytes that were signed
	sig: string; // 128-hex ed25519 signature
}

const NONCE_RE = /^[0-9a-fA-F]+$/;

/** Origin of a URL string, or undefined if it can't be parsed. */
function originOf(url: string): string | undefined {
	try {
		return new URL(url).origin;
	} catch {
		return undefined;
	}
}

/**
 * Validate the incoming /assert query params. `return_to` must be a URL whose
 * origin equals `audience`; `nonce` must be hex of 8–64 bytes (16–128 hex
 * chars); `prompt` defaults to "consent".
 */
export function parseAssertRequest(
	params: URLSearchParams
): AssertRequest | AssertRequestError {
	const audience = params.get('audience');
	const returnTo = params.get('return_to');
	const nonce = params.get('nonce');
	const promptRaw = params.get('prompt') ?? 'consent';

	if (!audience || !returnTo || !nonce) return { error: 'invalid_request' };
	if (promptRaw !== 'consent' && promptRaw !== 'none') return { error: 'invalid_request' };

	// audience must be a bare origin (no path/query), and must equal return_to's origin.
	const audienceOrigin = originOf(audience);
	if (!audienceOrigin || audienceOrigin !== audience) return { error: 'invalid_request' };
	if (originOf(returnTo) !== audience) return { error: 'invalid_request' };

	// nonce: hex, 8–64 bytes -> 16–128 hex chars, even length.
	if (!NONCE_RE.test(nonce)) return { error: 'invalid_request' };
	if (nonce.length % 2 !== 0 || nonce.length < 16 || nonce.length > 128) {
		return { error: 'invalid_request' };
	}

	return { audience, returnTo, nonce, prompt: promptRaw };
}

/**
 * Build the assertion payload JSON with EXACT key order. The object literal
 * below fixes the serialization order; both signer and verifier go through
 * this function so the bytes agree.
 */
export function buildAssertionPayload(args: {
	pubkey: string;
	displayName: string;
	audience: string;
	nonce: string;
	issuedAt: number;
}): string {
	const payload: AssertionPayload = {
		v: 1,
		pubkey: args.pubkey,
		display_name: args.displayName,
		audience: args.audience,
		nonce: args.nonce,
		issued_at: args.issuedAt,
		expires_at: args.issuedAt + ASSERT_TTL_SECS
	};
	return JSON.stringify(payload);
}

/** The exact bytes signed: domain prefix + newline + payload JSON. */
export function assertionSigningBytes(payloadJson: string): Uint8Array {
	return new TextEncoder().encode(`${ASSERT_DOMAIN}\n${payloadJson}`);
}

/** Sign the payload, returning the 128-hex signature. */
export function signAssertion(secretKey: Uint8Array, payloadJson: string): string {
	const sig = ed.sign(assertionSigningBytes(payloadJson), secretKey);
	return bytesToHex(sig);
}

function base64urlEncode(bytes: Uint8Array): string {
	let bin = '';
	for (const b of bytes) bin += String.fromCharCode(b);
	return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function base64urlDecode(s: string): Uint8Array {
	const pad = s.length % 4 === 0 ? '' : '='.repeat(4 - (s.length % 4));
	const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
	const bytes = new Uint8Array(bin.length);
	for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
	return bytes;
}

/** Encode {payload, sig} as base64url(JSON.stringify(...)) for the fragment. */
export function encodeAssertionToken(token: AssertionToken): string {
	return base64urlEncode(new TextEncoder().encode(JSON.stringify(token)));
}

/** Decode a base64url assertion token back to {payload, sig}. */
export function decodeAssertionToken(encoded: string): AssertionToken {
	const json = new TextDecoder().decode(base64urlDecode(encoded));
	const obj = JSON.parse(json);
	if (typeof obj?.payload !== 'string' || typeof obj?.sig !== 'string') {
		throw new Error('malformed assertion token');
	}
	return { payload: obj.payload, sig: obj.sig };
}

/** return_to + "#mjolnir_assertion=" + token (fragment only, never query). */
export function buildReturnUrl(returnTo: string, token: string): string {
	return `${returnTo}#mjolnir_assertion=${token}`;
}

/** return_to + "#mjolnir_assertion_error=" + code. */
export function buildErrorUrl(returnTo: string, code: AssertErrorCode): string {
	return `${returnTo}#mjolnir_assertion_error=${code}`;
}

export interface VerifyResult {
	ok: boolean;
	reason?: string;
	payload?: AssertionPayload;
}

/**
 * RP-side reference verifier (offline, self-certifying). Copy this into your
 * relying party. Verifies the signature over the exact carried payload bytes
 * with the pubkey INSIDE the payload, then checks audience, nonce, and expiry.
 *
 * @param encodedToken the base64url string from the `#mjolnir_assertion=` fragment
 * @param expected.audience  your own origin (must equal payload.audience)
 * @param expected.nonce     the single-use nonce you minted for this request
 * @param expected.now       current unix secs (defaults to Date.now()/1000)
 */
export function verifyAssertion(
	encodedToken: string,
	expected: { audience: string; nonce: string; now?: number }
): VerifyResult {
	let token: AssertionToken;
	try {
		token = decodeAssertionToken(encodedToken);
	} catch {
		return { ok: false, reason: 'malformed token' };
	}

	let payload: AssertionPayload;
	try {
		payload = JSON.parse(token.payload) as AssertionPayload;
	} catch {
		return { ok: false, reason: 'malformed payload' };
	}

	if (payload.v !== 1) return { ok: false, reason: 'unsupported version' };
	if (typeof payload.pubkey !== 'string' || !/^[0-9a-fA-F]{64}$/.test(payload.pubkey)) {
		return { ok: false, reason: 'bad pubkey' };
	}

	// Verify signature over the EXACT carried bytes with the embedded pubkey.
	let sigValid = false;
	try {
		sigValid = ed.verify(
			hexToBytes(token.sig),
			assertionSigningBytes(token.payload),
			hexToBytes(payload.pubkey)
		);
	} catch {
		return { ok: false, reason: 'bad signature encoding' };
	}
	if (!sigValid) return { ok: false, reason: 'signature mismatch' };

	if (payload.audience !== expected.audience) return { ok: false, reason: 'audience mismatch' };
	if (payload.nonce !== expected.nonce) return { ok: false, reason: 'nonce mismatch' };

	const now = expected.now ?? Math.floor(Date.now() / 1000);
	if (now >= payload.expires_at) return { ok: false, reason: 'expired' };

	return { ok: true, payload };
}
