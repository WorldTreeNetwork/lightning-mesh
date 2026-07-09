import { describe, expect, it } from 'vitest';
import { generateKeyPair, publicKeyHex } from './keys';
import {
	parseAssertRequest,
	buildAssertionPayload,
	signAssertion,
	encodeAssertionToken,
	decodeAssertionToken,
	buildReturnUrl,
	buildErrorUrl,
	verifyAssertion,
	ASSERT_TTL_SECS
} from './assert';

const AUDIENCE = 'http://wiki.mesh';
const RETURN_TO = 'http://wiki.mesh/callback';
const NONCE = 'deadbeefdeadbeef'; // 8 bytes hex

function params(overrides: Record<string, string | null> = {}): URLSearchParams {
	const base: Record<string, string> = {
		audience: AUDIENCE,
		return_to: RETURN_TO,
		nonce: NONCE,
		prompt: 'consent'
	};
	const p = new URLSearchParams();
	for (const [k, v] of Object.entries(base)) {
		if (overrides[k] === null) continue; // omit
		p.set(k, overrides[k] ?? v);
	}
	// allow adding keys not in base
	for (const [k, v] of Object.entries(overrides)) {
		if (v !== null && !(k in base)) p.set(k, v);
	}
	return p;
}

describe('parseAssertRequest', () => {
	it('accepts a valid request', () => {
		const r = parseAssertRequest(params());
		expect(r).toEqual({ audience: AUDIENCE, returnTo: RETURN_TO, nonce: NONCE, prompt: 'consent' });
	});

	it('defaults prompt to consent', () => {
		const r = parseAssertRequest(params({ prompt: null }));
		expect('error' in r).toBe(false);
		if (!('error' in r)) expect(r.prompt).toBe('consent');
	});

	it('rejects missing nonce', () => {
		expect(parseAssertRequest(params({ nonce: null }))).toEqual({ error: 'invalid_request' });
	});

	it('rejects when return_to origin !== audience', () => {
		expect(parseAssertRequest(params({ return_to: 'http://evil.mesh/callback' }))).toEqual({
			error: 'invalid_request'
		});
	});

	it('rejects audience carrying a path (not a bare origin)', () => {
		expect(
			parseAssertRequest(params({ audience: 'http://wiki.mesh/foo', return_to: 'http://wiki.mesh/foo' }))
		).toEqual({ error: 'invalid_request' });
	});

	it('rejects a non-hex nonce', () => {
		expect(parseAssertRequest(params({ nonce: 'zzzznotvalidhex!' }))).toEqual({
			error: 'invalid_request'
		});
	});

	it('rejects a too-short nonce (< 8 bytes)', () => {
		expect(parseAssertRequest(params({ nonce: 'deadbeef' }))).toEqual({ error: 'invalid_request' });
	});

	it('rejects an unknown prompt value', () => {
		expect(parseAssertRequest(params({ prompt: 'silent' }))).toEqual({ error: 'invalid_request' });
	});
});

describe('assertion round-trip', () => {
	async function mint(overrides: { issuedAt?: number; displayName?: string } = {}) {
		const { publicKey, secretKey } = await generateKeyPair();
		const pubkey = publicKeyHex(publicKey);
		const issuedAt = overrides.issuedAt ?? Math.floor(Date.now() / 1000);
		const payloadJson = buildAssertionPayload({
			pubkey,
			displayName: overrides.displayName ?? 'Ada',
			audience: AUDIENCE,
			nonce: NONCE,
			issuedAt
		});
		const sig = signAssertion(secretKey, payloadJson);
		const encoded = encodeAssertionToken({ payload: payloadJson, sig });
		return { pubkey, secretKey, payloadJson, sig, encoded, issuedAt };
	}

	it('emits payload with exact key order', async () => {
		const { payloadJson } = await mint();
		expect(Object.keys(JSON.parse(payloadJson))).toEqual([
			'v',
			'pubkey',
			'display_name',
			'audience',
			'nonce',
			'issued_at',
			'expires_at'
		]);
	});

	it('sets expires_at = issued_at + TTL', async () => {
		const { payloadJson, issuedAt } = await mint();
		expect(JSON.parse(payloadJson).expires_at).toBe(issuedAt + ASSERT_TTL_SECS);
	});

	it('happy path verifies', async () => {
		const { encoded, pubkey } = await mint();
		const res = verifyAssertion(encoded, { audience: AUDIENCE, nonce: NONCE });
		expect(res.ok).toBe(true);
		expect(res.payload?.pubkey).toBe(pubkey);
		expect(res.payload?.display_name).toBe('Ada');
	});

	it('rejects audience mismatch', async () => {
		const { encoded } = await mint();
		const res = verifyAssertion(encoded, { audience: 'http://other.mesh', nonce: NONCE });
		expect(res.ok).toBe(false);
		expect(res.reason).toBe('audience mismatch');
	});

	it('rejects nonce mismatch (replay with a different minted nonce)', async () => {
		const { encoded } = await mint();
		const res = verifyAssertion(encoded, { audience: AUDIENCE, nonce: 'aaaaaaaaaaaaaaaa' });
		expect(res.ok).toBe(false);
		expect(res.reason).toBe('nonce mismatch');
	});

	it('rejects an expired assertion', async () => {
		const past = Math.floor(Date.now() / 1000) - 10 * ASSERT_TTL_SECS;
		const { encoded } = await mint({ issuedAt: past });
		const res = verifyAssertion(encoded, { audience: AUDIENCE, nonce: NONCE });
		expect(res.ok).toBe(false);
		expect(res.reason).toBe('expired');
	});

	it('rejects a tampered payload (bad signature)', async () => {
		const { payloadJson, sig } = await mint();
		const tampered = payloadJson.replace('"Ada"', '"Mallory"');
		const encoded = encodeAssertionToken({ payload: tampered, sig });
		const res = verifyAssertion(encoded, { audience: AUDIENCE, nonce: NONCE });
		expect(res.ok).toBe(false);
		expect(res.reason).toBe('signature mismatch');
	});

	it('rejects a swapped signature from another key', async () => {
		const a = await mint();
		const b = await mint();
		const forged = encodeAssertionToken({ payload: a.payloadJson, sig: b.sig });
		const res = verifyAssertion(forged, { audience: AUDIENCE, nonce: NONCE });
		expect(res.ok).toBe(false);
		expect(res.reason).toBe('signature mismatch');
	});

	it('rejects a malformed token', () => {
		expect(verifyAssertion('!!!not-base64url!!!', { audience: AUDIENCE, nonce: NONCE }).ok).toBe(
			false
		);
	});

	it('decodes what it encodes', async () => {
		const { encoded, payloadJson, sig } = await mint();
		expect(decodeAssertionToken(encoded)).toEqual({ payload: payloadJson, sig });
	});
});

describe('redirect url builders', () => {
	it('carries the token in the fragment, never the query', () => {
		const url = buildReturnUrl(RETURN_TO, 'TOKEN123');
		expect(url).toBe('http://wiki.mesh/callback#mjolnir_assertion=TOKEN123');
		expect(url).not.toContain('?');
	});

	it('builds a deny/error url in the fragment', () => {
		expect(buildErrorUrl(RETURN_TO, 'access_denied')).toBe(
			'http://wiki.mesh/callback#mjolnir_assertion_error=access_denied'
		);
	});
});
