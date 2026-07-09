import { describe, expect, it } from 'vitest';
import { groupPeople, shortKey } from './people';
import type { DirectoryIdentity } from './api';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;
const NOW = 1_000_000_000_000;

function id(username: string, display_name: string, ageMs?: number): DirectoryIdentity {
	return {
		username,
		display_name,
		last_seen_unix: ageMs === undefined ? undefined : NOW - ageMs
	};
}

const K = (n: number) => n.toString(16).padStart(64, '0');

describe('shortKey', () => {
	it('takes the first 8 hex chars', () => {
		expect(shortKey('a1b2c3d4e5f6aabbcc')).toBe('a1b2c3d4');
	});
});

describe('groupPeople', () => {
	it('sorts present people most-recent-first', () => {
		const people = [
			id(K(1), 'Old', 3 * HOUR),
			id(K(2), 'Fresh', 30_000),
			id(K(3), 'Middle', 20 * MIN)
		];
		const { present, dormant } = groupPeople(people, undefined, NOW);
		expect(present.map((p) => p.displayName)).toEqual(['Fresh', 'Middle', 'Old']);
		expect(dormant).toHaveLength(0);
	});

	it('collapses identities older than a day into the dormant group', () => {
		const people = [id(K(1), 'Recent', 10 * MIN), id(K(2), 'Gone', 3 * DAY)];
		const { present, dormant } = groupPeople(people, undefined, NOW);
		expect(present.map((p) => p.displayName)).toEqual(['Recent']);
		expect(dormant.map((p) => p.displayName)).toEqual(['Gone']);
	});

	it('sorts unknown-recency people last and keeps them visible (not dormant)', () => {
		const people = [id(K(1), 'NoSeen'), id(K(2), 'Seen', 5 * MIN)];
		const { present, dormant } = groupPeople(people, undefined, NOW);
		expect(present.map((p) => p.displayName)).toEqual(['Seen', 'NoSeen']);
		expect(dormant).toHaveLength(0);
	});

	it('pins your own chip first even when your last announce is dormant', () => {
		const self = K(9);
		const people = [
			id(K(1), 'Active', 30_000),
			id(self, 'Me', 5 * DAY) // dormant age, but it's us
		];
		const { present, dormant } = groupPeople(people, self, NOW);
		expect(present[0].isSelf).toBe(true);
		expect(present[0].displayName).toBe('Me');
		expect(present.map((p) => p.displayName)).toEqual(['Me', 'Active']);
		expect(dormant).toHaveLength(0);
	});

	it('matches the self key case-insensitively', () => {
		const self = 'ABCDEF'.padEnd(64, '0');
		const people = [id(self.toLowerCase(), 'Me', 1 * MIN)];
		const { present } = groupPeople(people, self, NOW);
		expect(present[0].isSelf).toBe(true);
	});
});
