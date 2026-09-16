import type { AssertErrorCode } from '$lib/identity/assert';

const NONCE_RE = /^[0-9a-fA-F]+$/;
export const PENDING_MS = 60_000;
export const COOLDOWN_START_MS = 30_000;
export const COOLDOWN_MAX_MS = 10 * 60_000;

export type PromptKind = 'consent' | 'none';

export interface IdentityRequestInput {
	eventSource: unknown;
	eventOrigin: string;
	cardSource: unknown;
	entryOrigin: string;
	portCount: number;
	nonce: unknown;
	prompt: unknown;
	serviceName: string;
	cardId: string;
	hasKey: boolean;
	approved: boolean;
	now: number;
}

export type IdentityDecision =
	| { kind: 'drop' }
	| {
			kind: 'respond';
			error?: AssertErrorCode;
			echoNonce?: string;
			sign?: boolean;
			prompt?: boolean;
			offerCreate?: boolean;
			replacePending?: PendingRecord;
			expirePending?: true;
			startCooldown?: boolean;
			resetCooldown?: boolean;
	  };

export interface PendingRecord {
	cardId: string;
	serviceName: string;
	origin: string;
	nonce: string;
	deadline: number;
	claimed: boolean;
}

export interface CooldownRecord {
	until: number;
	nextMs: number;
}

function parseNonce(nonce: unknown): string | undefined {
	if (typeof nonce !== 'string' || !NONCE_RE.test(nonce)) return undefined;
	if (nonce.length % 2 !== 0 || nonce.length < 16 || nonce.length > 128) return undefined;
	return nonce;
}

function parsePrompt(prompt: unknown): PromptKind | undefined {
	if (prompt === undefined || prompt === null) return 'consent';
	if (prompt === 'consent' || prompt === 'none') return prompt;
	return undefined;
}

export function isFramed(win: { top: unknown; self: unknown }): boolean {
	try {
		return win.top !== win.self;
	} catch {
		return true;
	}
}

export function nextCooldownMs(previous = 0): number {
	if (previous <= 0) return COOLDOWN_START_MS;
	return Math.min(previous * 2, COOLDOWN_MAX_MS);
}

export function decideIdentityRequest(
	input: IdentityRequestInput,
	pending: PendingRecord | null,
	cooldown: CooldownRecord | null
): IdentityDecision {
	if (input.eventOrigin === 'null' || !input.eventOrigin) return { kind: 'drop' };
	if (input.eventSource !== input.cardSource) return { kind: 'drop' };
	if (input.eventOrigin !== input.entryOrigin) return { kind: 'drop' };
	if (input.portCount !== 1) return { kind: 'drop' };

	const nonce = parseNonce(input.nonce);
	const prompt = parsePrompt(input.prompt);
	if (!nonce || !prompt) {
		return {
			kind: 'respond',
			error: 'invalid_request',
			echoNonce: nonce
		};
	}

	let expirePending = false;
	let current = pending;
	if (current && !current.claimed && input.now >= current.deadline) {
		expirePending = true;
		current = null;
	}

	if (current && !current.claimed && current.cardId === input.cardId) {
		return expirePending
			? { kind: 'respond', error: 'interaction_required', echoNonce: nonce, expirePending: true }
			: { kind: 'drop' };
	}

	if (current && !current.claimed) {
		return {
			kind: 'respond',
			error: 'interaction_required',
			echoNonce: nonce,
			expirePending: expirePending || undefined
		};
	}

	if (cooldown && input.now < cooldown.until) {
		return {
			kind: 'respond',
			error: 'interaction_required',
			echoNonce: nonce,
			expirePending: expirePending || undefined
		};
	}

	if (prompt === 'none') {
		if (input.hasKey && input.approved) {
			return {
				kind: 'respond',
				sign: true,
				echoNonce: nonce,
				resetCooldown: true,
				expirePending: expirePending || undefined,
				replacePending: {
					cardId: input.cardId,
					serviceName: input.serviceName,
					origin: input.entryOrigin,
					nonce,
					deadline: input.now + PENDING_MS,
					claimed: true
				}
			};
		}
		return {
			kind: 'respond',
			error: 'interaction_required',
			echoNonce: nonce,
			expirePending: expirePending || undefined
		};
	}

	return {
		kind: 'respond',
		prompt: true,
		offerCreate: !input.hasKey,
		echoNonce: nonce,
		expirePending: expirePending || undefined,
		replacePending: {
			cardId: input.cardId,
			serviceName: input.serviceName,
			origin: input.entryOrigin,
			nonce,
			deadline: input.now + PENDING_MS,
			claimed: false
		}
	};
}

export function decideAllow(
	now: number,
	pending: PendingRecord | null
): { kind: 'drop' } | { kind: 'expire' } | { kind: 'sign'; record: PendingRecord } {
	if (!pending || pending.claimed) return { kind: 'drop' };
	if (now >= pending.deadline) return { kind: 'expire' };
	return { kind: 'sign', record: { ...pending, claimed: true } };
}

export function decideDeny(pending: PendingRecord | null): PendingRecord | null {
	if (!pending || pending.claimed) return null;
	return pending;
}

export interface ResponsePort {
	postMessage(data: unknown): void;
	close(): void;
}

function respondOnPort(
	port: ResponsePort | null,
	nonce: string | undefined,
	body: { token?: string; error?: AssertErrorCode }
): void {
	if (!port) return;
	const message: Record<string, unknown> = {
		mesh: 'mini-app/v1',
		type: 'identity.response'
	};
	if (nonce) message.nonce = nonce;
	if (body.token) message.token = body.token;
	if (body.error) message.error = body.error;
	try {
		port.postMessage(message);
	} finally {
		port.close();
	}
}

export class IdentityBridgeHost {
	pending: PendingRecord | null = null;
	port: ResponsePort | null = null;
	sheet: { origin: string; nonce: string; offerCreate: boolean; name?: string } | null = null;
	sharedWith: string | null = null;
	cooldowns = new Map<string, CooldownRecord>();

	applyCooldown(serviceName: string, now: number): void {
		const prev = this.cooldowns.get(serviceName);
		const nextMs = nextCooldownMs(prev?.nextMs ?? 0);
		this.cooldowns.set(serviceName, { until: now + nextMs, nextMs });
	}

	resetCooldown(serviceName: string): void {
		this.cooldowns.delete(serviceName);
	}

	endPending(error: AssertErrorCode, startCooldown: boolean, now: number): void {
		const pending = this.pending;
		const port = this.port;
		this.pending = null;
		this.port = null;
		this.sheet = null;
		respondOnPort(port, pending?.nonce, { error });
		if (startCooldown && pending) this.applyCooldown(pending.serviceName, now);
	}

	handleRequest(
		input: IdentityRequestInput,
		port: ResponsePort | null,
		appName?: string
	): void {
		const decision = decideIdentityRequest(
			input,
			this.pending,
			this.cooldowns.get(input.serviceName) ?? null
		);
		if (decision.kind === 'drop') return;
		if (decision.expirePending && this.pending && !this.pending.claimed) {
			this.endPending('interaction_required', true, input.now);
		}
		if (decision.replacePending) {
			this.pending = decision.replacePending;
			this.port = port;
		}
		if (decision.sign) {
			return;
		}
		if (decision.prompt) {
			this.sheet = {
				origin: input.entryOrigin,
				nonce: decision.echoNonce ?? '',
				offerCreate: Boolean(decision.offerCreate),
				name: appName
			};
			return;
		}
		if (decision.error) {
			respondOnPort(port, decision.echoNonce, { error: decision.error });
		}
	}

	claimAllow(now: number): PendingRecord | null {
		const d = decideAllow(now, this.pending);
		if (d.kind === 'expire') {
			this.endPending('interaction_required', true, now);
			return null;
		}
		if (d.kind === 'drop') return null;
		this.pending = d.record;
		this.sheet = null;
		return d.record;
	}

	completeSign(token: string): void {
		const pending = this.pending;
		const port = this.port;
		this.pending = null;
		this.port = null;
		if (pending) {
			this.resetCooldown(pending.serviceName);
			this.sharedWith = pending.origin;
		}
		respondOnPort(port, pending?.nonce, { token });
	}

	deny(now: number): void {
		if (decideDeny(this.pending)) this.endPending('access_denied', true, now);
	}

	dismiss(now: number): void {
		if (this.pending && !this.pending.claimed) this.endPending('interaction_required', true, now);
	}

	iframeLoaded(cardId: string, now: number): void {
		this.sharedWith = null;
		if (this.pending && !this.pending.claimed && this.pending.cardId === cardId) {
			this.endPending('interaction_required', true, now);
		}
	}

	cardRemoved(cardId: string, now: number): void {
		this.sharedWith = null;
		if (this.pending && this.pending.cardId === cardId && !this.pending.claimed) {
			this.endPending('interaction_required', false, now);
		}
	}

	trustedOpen(serviceName: string): void {
		this.resetCooldown(serviceName);
	}
}
