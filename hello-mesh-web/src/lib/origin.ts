// Rung-1 keys live in IndexedDB keyed to the page ORIGIN, so an identity made
// at http://hello.mesh is invisible when the same node is reached by raw IP.
// This detects the "you're here by IP" case so the UI can nudge the user back
// to the canonical name. Dev/localhost is exempt so the mock/dev server is quiet.

export const CANONICAL_HOST = 'hello.mesh';

const DEV_HOSTS = new Set(['localhost', '127.0.0.1', '::1', '0.0.0.0']);

/** True when the user is on a non-canonical, non-dev origin (typically a raw IP). */
export function isForeignOrigin(hostname: string): boolean {
	const h = hostname.toLowerCase();
	if (h === CANONICAL_HOST) return false;
	if (DEV_HOSTS.has(h)) return false;
	return true;
}

/** The canonical URL to steer the user toward, preserving the current scheme. */
export function canonicalUrl(protocol: string): string {
	return `${protocol}//${CANONICAL_HOST}`;
}
