import { describe, expect, it } from 'vitest';
import { presenceTier, recencyLabel, ACTIVE_WINDOW_MS } from './presence';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

describe('presenceTier', () => {
	it('classifies by age into the four decaying tiers', () => {
		expect(presenceTier(0)).toBe('active');
		expect(presenceTier(ACTIVE_WINDOW_MS - 1)).toBe('active');
		expect(presenceTier(ACTIVE_WINDOW_MS + 1)).toBe('recent');
		expect(presenceTier(30 * MIN)).toBe('recent');
		expect(presenceTier(HOUR + 1)).toBe('today');
		expect(presenceTier(5 * HOUR)).toBe('today');
		expect(presenceTier(DAY + 1)).toBe('stale');
		expect(presenceTier(10 * DAY)).toBe('stale');
	});

	it('treats a missing timestamp as unknown, not active', () => {
		expect(presenceTier(undefined)).toBe('unknown');
		expect(presenceTier(NaN)).toBe('unknown');
	});

	it('clamps mild negative skew (writer clock ahead of ours) to active', () => {
		expect(presenceTier(-2000)).toBe('active');
	});
});

describe('recencyLabel', () => {
	it('reads as plain language per tier', () => {
		expect(recencyLabel(30_000)).toBe('here now');
		expect(recencyLabel(12 * MIN)).toBe('seen 12m ago');
		expect(recencyLabel(3 * HOUR)).toBe('seen 3h ago');
		expect(recencyLabel(2 * DAY)).toBe('seen 2d ago');
	});

	it('never shows 0m — rounds sub-minute recent ages up to 1m', () => {
		expect(recencyLabel(ACTIVE_WINDOW_MS + 1000)).toBe('seen 5m ago');
	});

	it('is empty when recency is unknown', () => {
		expect(recencyLabel(undefined)).toBe('');
	});
});
