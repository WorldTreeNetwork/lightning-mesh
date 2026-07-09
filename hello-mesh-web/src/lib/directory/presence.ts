// Recency presence for people in the directory. We never claim someone is
// "online" — the directory has no liveness for identities, only a last-announce
// timestamp (last_seen_unix, writer clock, approximate). So presence is a soft,
// decaying "seen recently" signal, bucketed into a few tiers the UI renders as
// a colored dot + a plain-language label. Missing last_seen_unix => unknown.

export type PresenceTier = 'active' | 'recent' | 'today' | 'stale' | 'unknown';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

/** Anyone seen within this window gets a nudge re-announce / "here now" dot. */
export const ACTIVE_WINDOW_MS = 5 * MIN;
/** Identities not seen within this window collapse into a "not seen recently" group. */
export const DORMANT_WINDOW_MS = DAY;

/**
 * Classify an identity by how long ago it last announced.
 * `age` is `Date.now() - last_seen_unix` in ms; pass `undefined` when the
 * daemon didn't report last_seen_unix at all.
 */
export function presenceTier(age: number | undefined): PresenceTier {
	if (age === undefined || Number.isNaN(age)) return 'unknown';
	// Clamp mild clock skew (writer clock can be slightly ahead of ours).
	const a = Math.max(0, age);
	if (a < ACTIVE_WINDOW_MS) return 'active';
	if (a < HOUR) return 'recent';
	if (a < DAY) return 'today';
	return 'stale';
}

/** Short human recency label, e.g. "here now", "seen 12m ago", "seen 3d ago". */
export function recencyLabel(age: number | undefined): string {
	const tier = presenceTier(age);
	if (tier === 'unknown') return '';
	if (tier === 'active') return 'here now';
	const a = Math.max(0, age as number);
	if (a < HOUR) return `seen ${Math.max(1, Math.round(a / MIN))}m ago`;
	if (a < DAY) return `seen ${Math.round(a / HOUR)}h ago`;
	return `seen ${Math.round(a / DAY)}d ago`;
}

/** The `--presence-*` custom property a tier's dot should paint with. */
export function presenceColorVar(tier: PresenceTier): string {
	switch (tier) {
		case 'active':
			return 'var(--presence-active)';
		case 'recent':
			return 'var(--presence-recent)';
		case 'today':
			return 'var(--presence-today)';
		default:
			return 'var(--presence-stale)';
	}
}
