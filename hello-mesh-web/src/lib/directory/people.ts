// Presentation model for the People chip wall. Turns the raw directory
// identities (LWW pubkey -> display_name, plus an approximate last_seen_unix)
// into sorted, tiered "person" rows: your own chip pinned first, everyone seen
// recently next (most-recent first), and dormant identities folded into a
// collapsible group so the directory stops looking like it "never forgets".

import type { DirectoryIdentity } from './api';
import { DORMANT_WINDOW_MS, presenceTier, recencyLabel, type PresenceTier } from './presence';

export interface Person {
	/** Raw 64-hex public key (the directory `username`). Never rendered inline. */
	key: string;
	/** Chosen display name, or a short-key fallback the daemon already applies. */
	displayName: string;
	/** First 8 hex chars — the muted mono handle shown on the chip. */
	shortKey: string;
	lastSeenUnix?: number;
	/** Age in ms at the time the list was built; undefined when unknown. */
	age?: number;
	tier: PresenceTier;
	recency: string;
	isSelf: boolean;
}

export interface PeopleGroups {
	/** Your chip (if you're in the directory) followed by recently-seen people. */
	present: Person[];
	/** Identities not seen within the dormant window, hidden behind a disclosure. */
	dormant: Person[];
}

export function shortKey(key: string): string {
	return key.length > 8 ? key.slice(0, 8) : key;
}

function toPerson(id: DirectoryIdentity, selfKey: string | undefined, now: number): Person {
	const age = id.last_seen_unix === undefined ? undefined : now - id.last_seen_unix;
	return {
		key: id.username,
		displayName: id.display_name,
		shortKey: shortKey(id.username),
		lastSeenUnix: id.last_seen_unix,
		age,
		tier: presenceTier(age),
		recency: recencyLabel(age),
		isSelf: !!selfKey && id.username.toLowerCase() === selfKey.toLowerCase()
	};
}

/**
 * Most-recent-first ordering. Known ages sort ascending (smaller age = more
 * recent = earlier); identities with no last_seen sort last, then by name so
 * the list is stable.
 */
function byRecency(a: Person, b: Person): number {
	const aKnown = a.age !== undefined;
	const bKnown = b.age !== undefined;
	if (aKnown && bKnown) {
		if (a.age !== b.age) return (a.age as number) - (b.age as number);
		return a.displayName.localeCompare(b.displayName);
	}
	if (aKnown) return -1;
	if (bKnown) return 1;
	return a.displayName.localeCompare(b.displayName);
}

/**
 * Build the grouped chip-wall model. `selfKey` is your public-key hex (or
 * undefined when anonymous); your chip is always pinned first and never
 * collapsed, even if your own last announce has gone dormant.
 */
export function groupPeople(
	identities: DirectoryIdentity[],
	selfKey: string | undefined,
	now: number = Date.now()
): PeopleGroups {
	const people = identities.map((id) => toPerson(id, selfKey, now));

	const self = people.find((p) => p.isSelf);
	const others = people.filter((p) => !p.isSelf);

	const present: Person[] = [];
	const dormant: Person[] = [];
	for (const p of others) {
		// Unknown-recency people stay visible (we can't prove they're dormant).
		const isDormant = p.age !== undefined && p.age >= DORMANT_WINDOW_MS;
		(isDormant ? dormant : present).push(p);
	}

	present.sort(byRecency);
	dormant.sort(byRecency);

	return {
		present: self ? [self, ...present] : present,
		dormant
	};
}
