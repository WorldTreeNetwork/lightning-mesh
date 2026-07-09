import { describe, expect, it } from 'vitest';
import { deriveStatus, statusMessage, ANNOUNCE_SLOW_MS, type StatusInputs } from './status';

function inputs(over: Partial<StatusInputs> = {}): StatusInputs {
	return {
		haveLocalKey: true,
		inDirectory: false,
		nameInSync: false,
		lastAnnounceErrored: false,
		announcingForMs: 0,
		...over
	};
}

describe('deriveStatus', () => {
	it('is anonymous only when there is no local key', () => {
		expect(deriveStatus(inputs({ haveLocalKey: false }))).toBe('anonymous');
	});

	it('never returns anonymous when a key exists (the forgot-my-key bug)', () => {
		// Key present but not yet in the directory: announcing, NOT "create one".
		expect(deriveStatus(inputs({ haveLocalKey: true, inDirectory: false }))).toBe('announcing');
	});

	it('is present when in the directory with a matching name', () => {
		expect(deriveStatus(inputs({ inDirectory: true, nameInSync: true }))).toBe('present');
	});

	it('is syncing-name when in the directory but the name differs', () => {
		expect(deriveStatus(inputs({ inDirectory: true, nameInSync: false }))).toBe('syncing-name');
	});

	it('escalates to announcing-slow past the slow threshold', () => {
		expect(deriveStatus(inputs({ announcingForMs: ANNOUNCE_SLOW_MS + 1 }))).toBe('announcing-slow');
	});

	it('surfaces error only while still absent from the directory', () => {
		expect(deriveStatus(inputs({ lastAnnounceErrored: true }))).toBe('error');
		// Once we're in the directory, a stale prior error no longer matters.
		expect(
			deriveStatus(inputs({ inDirectory: true, nameInSync: true, lastAnnounceErrored: true }))
		).toBe('present');
	});
});

describe('statusMessage', () => {
	it('greets a settled identity by name', () => {
		expect(statusMessage('present', 'Maya')).toBe("You're Maya here");
	});

	it('has non-empty copy for every state', () => {
		for (const s of [
			'anonymous',
			'announcing',
			'announcing-slow',
			'syncing-name',
			'present',
			'error'
		] as const) {
			expect(statusMessage(s, 'Maya').length).toBeGreaterThan(0);
		}
	});
});
