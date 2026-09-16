import { describe, expect, it } from 'vitest';
import { generateKeyPair, publicKeyHex } from '$lib/identity/keys';
import {
	buildAssertionPayload,
	encodeAssertionToken,
	signAssertion,
	verifyAssertion
} from '$lib/identity/assert';
import {
	COOLDOWN_MAX_MS,
	COOLDOWN_START_MS,
	IdentityBridgeHost,
	PENDING_MS,
	decideAllow,
	decideIdentityRequest,
	isFramed,
	nextCooldownMs,
	type IdentityRequestInput,
	type ResponsePort
} from './identity-bridge';

const card = {};
const other = {};

function req(overrides: Partial<IdentityRequestInput> = {}): IdentityRequestInput {
	return {
		eventSource: card,
		eventOrigin: 'http://keyed.mesh:3000',
		cardSource: card,
		entryOrigin: 'http://keyed.mesh:3000',
		portCount: 1,
		nonce: 'aabbccddeeff0011',
		prompt: 'consent',
		serviceName: 'keyed',
		cardId: 'keyed|http|10.42.7.20|3000',
		hasKey: true,
		approved: false,
		now: 1_000,
		...overrides
	};
}

describe('decideIdentityRequest', () => {
	it('audience is the entry origin, extra audience field ignored', () => {
		const d = decideIdentityRequest(req(), null, null);
		expect(d.kind).toBe('respond');
		if (d.kind === 'respond') {
			expect(d.prompt).toBe(true);
			expect(d.replacePending?.origin).toBe('http://keyed.mesh:3000');
		}
	});

	it('malformed nonce with a port is invalid_request without echo', () => {
		const d = decideIdentityRequest(req({ nonce: 'xyz' }), null, null);
		expect(d).toMatchObject({ kind: 'respond', error: 'invalid_request' });
		if (d.kind === 'respond') expect(d.echoNonce).toBeUndefined();
	});

	it('no port is dropped', () => {
		expect(decideIdentityRequest(req({ portCount: 0 }), null, null)).toEqual({ kind: 'drop' });
		expect(decideIdentityRequest(req({ portCount: 2 }), null, null)).toEqual({ kind: 'drop' });
	});

	it('foreign origin is dropped', () => {
		expect(decideIdentityRequest(req({ eventOrigin: 'http://other.mesh' }), null, null)).toEqual({
			kind: 'drop'
		});
	});

	it('duplicate from the same card is dropped', () => {
		const first = decideIdentityRequest(req(), null, null);
		expect(first.kind).toBe('respond');
		if (first.kind !== 'respond' || !first.replacePending) throw new Error('expected pending');
		const dup = decideIdentityRequest(req(), first.replacePending, null);
		expect(dup).toEqual({ kind: 'drop' });
	});

	it('second card is busy while first is pending', () => {
		const first = decideIdentityRequest(req(), null, null);
		if (first.kind !== 'respond' || !first.replacePending) throw new Error('expected pending');
		const d = decideIdentityRequest(
			req({
				cardId: 'guestbook|http|10.42.7.21|80',
				serviceName: 'guestbook',
				eventOrigin: 'http://guestbook.mesh',
				entryOrigin: 'http://guestbook.mesh',
				cardSource: other,
				eventSource: other
			}),
			first.replacePending,
			null
		);
		expect(d).toMatchObject({ kind: 'respond', error: 'interaction_required' });
	});

	it('overdue pending expires before a new request or Allow', () => {
		const first = decideIdentityRequest(req(), null, null);
		if (first.kind !== 'respond' || !first.replacePending) throw new Error('expected pending');
		const later = first.replacePending.deadline + 1;
		const fromB = decideIdentityRequest(
			req({
				now: later,
				cardId: 'guestbook|http|10.42.7.21|80',
				serviceName: 'guestbook',
				eventOrigin: 'http://guestbook.mesh',
				entryOrigin: 'http://guestbook.mesh',
				cardSource: other,
				eventSource: other
			}),
			first.replacePending,
			null
		);
		expect(fromB.kind).toBe('respond');
		if (fromB.kind === 'respond') {
			expect(fromB.expirePending).toBe(true);
			expect(fromB.prompt).toBe(true);
		}
		expect(decideAllow(later, first.replacePending)).toEqual({ kind: 'expire' });
	});

	it('Allow just before the deadline claims the record', () => {
		const first = decideIdentityRequest(req(), null, null);
		if (first.kind !== 'respond' || !first.replacePending) throw new Error('expected pending');
		const d = decideAllow(first.replacePending.deadline - 1, first.replacePending);
		expect(d.kind).toBe('sign');
		if (d.kind === 'sign') expect(d.record.claimed).toBe(true);
	});

	it('prompt none without approval is interaction_required', () => {
		const d = decideIdentityRequest(req({ prompt: 'none' }), null, null);
		expect(d).toMatchObject({ kind: 'respond', error: 'interaction_required' });
	});

	it('prompt none with key and approval signs', () => {
		const d = decideIdentityRequest(req({ prompt: 'none', approved: true }), null, null);
		expect(d.kind).toBe('respond');
		if (d.kind === 'respond') expect(d.sign).toBe(true);
	});

	it('cooldown blocks without opening a sheet', () => {
		const d = decideIdentityRequest(req(), null, {
			until: 1_000 + COOLDOWN_START_MS,
			nextMs: COOLDOWN_START_MS
		});
		expect(d).toMatchObject({ kind: 'respond', error: 'interaction_required' });
		if (d.kind === 'respond') expect(d.prompt).toBeUndefined();
	});

	it('one app cooldown does not throttle another name', () => {
		const d = decideIdentityRequest(
			req({
				serviceName: 'guestbook',
				cardId: 'guestbook|http|10.42.7.21|80',
				eventOrigin: 'http://guestbook.mesh',
				entryOrigin: 'http://guestbook.mesh',
				cardSource: other,
				eventSource: other
			}),
			null,
			null
		);
		expect(d.kind).toBe('respond');
		if (d.kind === 'respond') expect(d.prompt).toBe(true);
	});
});

describe('cooldown and framing', () => {
	it('doubles up to ten minutes', () => {
		expect(nextCooldownMs(0)).toBe(COOLDOWN_START_MS);
		expect(nextCooldownMs(COOLDOWN_START_MS)).toBe(60_000);
		expect(nextCooldownMs(COOLDOWN_MAX_MS)).toBe(COOLDOWN_MAX_MS);
	});

	it('isFramed is true when top is not self', () => {
		expect(isFramed({ top: {}, self: {} })).toBe(true);
		const w = {};
		expect(isFramed({ top: w, self: w })).toBe(false);
	});

	it('pending window is 60s', () => {
		expect(PENDING_MS).toBe(60_000);
	});
});

class FakePort implements ResponsePort {
	posted: unknown[] = [];
	closed = false;
	started = false;
	onmessage: ((ev: unknown) => void) | undefined;
	start(): void {
		this.started = true;
	}
	postMessage(data: unknown): void {
		this.posted.push(data);
	}
	close(): void {
		this.closed = true;
	}
}

describe('IdentityBridgeHost', () => {
	it('never starts the port and ignores forged follow-ups', () => {
		const host = new IdentityBridgeHost();
		const port = new FakePort();
		host.handleRequest(req(), port);
		expect(port.started).toBe(false);
		expect(port.onmessage).toBeUndefined();
		const nonce = host.pending?.nonce;
		const origin = host.pending?.origin;
		port.onmessage?.({ nonce: 'ffffffffffffffff', audience: 'http://bank.mesh' });
		expect(host.pending?.nonce).toBe(nonce);
		expect(host.pending?.origin).toBe(origin);
		const claimed = host.claimAllow(1_000);
		expect(claimed?.nonce).toBe(nonce);
		host.completeSign('tok');
		expect(port.posted).toHaveLength(1);
		expect(port.posted[0]).toMatchObject({
			mesh: 'mini-app/v1',
			type: 'identity.response',
			nonce,
			token: 'tok'
		});
		expect(port.closed).toBe(true);
		host.completeSign('tok2');
		expect(port.posted).toHaveLength(1);
	});

	it('deny is access_denied; dismiss is interaction_required; both start cooldown', () => {
		const denied = new IdentityBridgeHost();
		const denyPort = new FakePort();
		denied.handleRequest(req(), denyPort);
		denied.deny(1_000);
		expect(denyPort.posted[0]).toMatchObject({ error: 'access_denied' });
		expect(denied.cooldowns.get('keyed')?.until).toBe(1_000 + COOLDOWN_START_MS);

		const dismissed = new IdentityBridgeHost();
		const dismissPort = new FakePort();
		dismissed.handleRequest(req(), dismissPort);
		dismissed.dismiss(1_000);
		expect(dismissPort.posted[0]).toMatchObject({ error: 'interaction_required' });
		expect(dismissed.cooldowns.get('keyed')?.until).toBe(1_000 + COOLDOWN_START_MS);
	});

	it('cooldown survives ip and port republish of the same service name', () => {
		const host = new IdentityBridgeHost();
		host.applyCooldown('keyed', 1_000);
		const port = new FakePort();
		host.handleRequest(
			req({
				cardId: 'keyed|http|10.42.9.9|80',
				now: 1_001
			}),
			port
		);
		expect(port.posted[0]).toMatchObject({ error: 'interaction_required' });
		expect(host.sheet).toBeNull();
		const other = new FakePort();
		host.handleRequest(
			req({
				serviceName: 'guestbook',
				cardId: 'guestbook|http|10.42.7.21|80',
				eventOrigin: 'http://guestbook.mesh',
				entryOrigin: 'http://guestbook.mesh',
				cardSource: other,
				eventSource: other,
				now: 1_001
			}),
			other
		);
		expect(host.sheet?.origin).toBe('http://guestbook.mesh');
	});

	it('trusted open resets cooldown; iframe load does not', () => {
		const host = new IdentityBridgeHost();
		const port = new FakePort();
		host.handleRequest(req(), port);
		host.iframeLoaded(req().cardId, 1_000);
		expect(port.posted[0]).toMatchObject({ error: 'interaction_required' });
		expect(host.cooldowns.has('keyed')).toBe(true);
		const blocked = new FakePort();
		host.handleRequest(req({ nonce: 'aabbccddeeff0022', now: 1_001 }), blocked);
		expect(blocked.posted[0]).toMatchObject({ error: 'interaction_required' });
		host.trustedOpen('keyed');
		const allowed = new FakePort();
		host.handleRequest(req({ nonce: 'aabbccddeeff0033', now: 1_002 }), allowed);
		expect(host.sheet).not.toBeNull();
		expect(allowed.posted).toHaveLength(0);
	});

	it('card removal ends pending without starting cooldown', () => {
		const host = new IdentityBridgeHost();
		const port = new FakePort();
		host.handleRequest(req(), port);
		host.cardRemoved(req().cardId, 1_000);
		expect(port.posted[0]).toMatchObject({ error: 'interaction_required' });
		expect(host.cooldowns.size).toBe(0);
	});

	it('prompt none token round-trips through verifyAssertion', async () => {
		const host = new IdentityBridgeHost();
		const port = new FakePort();
		const keys = await generateKeyPair();
		const origin = 'http://keyed.mesh:3000';
		const nonce = 'aabbccddeeff0011';
		host.handleRequest(req({ prompt: 'none', approved: true, hasKey: true }), port);
		expect(host.pending?.claimed).toBe(true);
		expect(host.pending?.origin).toBe(origin);
		const payloadJson = buildAssertionPayload({
			pubkey: publicKeyHex(keys.publicKey),
			displayName: 'Ada',
			audience: origin,
			nonce,
			issuedAt: Math.floor(Date.now() / 1000)
		});
		const token = encodeAssertionToken({
			payload: payloadJson,
			sig: signAssertion(keys.secretKey, payloadJson)
		});
		host.completeSign(token);
		expect(port.posted).toHaveLength(1);
		const body = port.posted[0] as { token: string; nonce: string };
		expect(body.nonce).toBe(nonce);
		const verified = verifyAssertion(body.token, { audience: origin, nonce });
		expect(verified.ok).toBe(true);
		expect(verified.payload?.audience).toBe(origin);
		expect(host.sharedWith).toBe(origin);
	});
});
