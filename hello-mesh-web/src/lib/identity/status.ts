// Identity status is DERIVED from the directory every poll, not stored as a
// one-shot flag — that's the fix for "the app forgot my key and told me to
// create one again". Inputs: whether a key exists locally (IndexedDB), whether
// that key is present in the directory the node just served, and whether the
// directory's display_name matches the label we saved. From those we compute a
// single user-facing state. We NEVER surface an "online" boolean.

export type IdentityStatus =
	| 'anonymous' // no local key
	| 'announcing' // have a key, not yet in the directory (introducing)
	| 'announcing-slow' // ditto, but it's been long enough to auto re-announce
	| 'syncing-name' // in the directory, but display_name != our label
	| 'present' // in the directory and the name matches — settled
	| 'error'; // last announce attempt threw; manual retry offered

export interface StatusInputs {
	haveLocalKey: boolean;
	inDirectory: boolean;
	/** directory display_name === stored label (only meaningful when inDirectory). */
	nameInSync: boolean;
	/** Last announce attempt errored and we're not yet back in the directory. */
	lastAnnounceErrored: boolean;
	/** ms since we first started trying to announce this key, or undefined. */
	announcingForMs: number | undefined;
}

/** Past this, a still-absent key auto re-announces (network hiccup / lost spool). */
export const ANNOUNCE_SLOW_MS = 60_000;

export function deriveStatus(inputs: StatusInputs): IdentityStatus {
	if (!inputs.haveLocalKey) return 'anonymous';
	if (inputs.inDirectory) {
		return inputs.nameInSync ? 'present' : 'syncing-name';
	}
	// Have a key but the directory doesn't show it yet.
	if (inputs.lastAnnounceErrored) return 'error';
	if (inputs.announcingForMs !== undefined && inputs.announcingForMs > ANNOUNCE_SLOW_MS) {
		return 'announcing-slow';
	}
	return 'announcing';
}

/** One-line status message for the given state; `name` is the settled display name. */
export function statusMessage(status: IdentityStatus, name: string): string {
	switch (status) {
		case 'anonymous':
			return 'Browsing anonymously';
		case 'announcing':
			return 'Introducing you to the mesh — this takes up to about 20 seconds.';
		case 'announcing-slow':
			return 'Still introducing you — retrying. This can take a moment on a busy mesh.';
		case 'syncing-name':
			return 'Syncing your name across the mesh…';
		case 'present':
			return name ? `You're ${name} here` : "You're on the mesh";
		case 'error':
			return "Couldn't reach this node to announce you. Try again.";
	}
}
