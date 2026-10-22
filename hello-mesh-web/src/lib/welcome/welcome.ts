// House-hub welcome: pure, UI-local logic for the identity invitation.
// Nothing here talks to the network, routing, firewall, CAPPORT, or the
// identity store. The only path to identity creation is `confirm` from the
// consent phase; every other action is a no-op on identity.

export type Phase = 'invited' | 'consenting' | 'creating' | 'anonymous' | 'dismissed' | 'internet';
export type Action =
	'explore' | 'dismiss' | 'justInternet' | 'startCreate' | 'cancelConsent' | 'confirm';

export interface InvitationState {
	phase: Phase;
}

export const initialInvitation: InvitationState = { phase: 'invited' };

export function reduceInvitation(state: InvitationState, action: Action): InvitationState {
	switch (action) {
		case 'explore':
			return { phase: 'anonymous' };
		case 'dismiss':
			return { phase: 'dismissed' };
		case 'justInternet':
			return { phase: 'internet' };
		case 'startCreate':
			return { phase: 'consenting' };
		case 'cancelConsent':
			return { phase: 'invited' };
		case 'confirm':
			return state.phase === 'consenting' ? { phase: 'creating' } : state;
	}
}

/** True only for the one phase where the caller may create an identity. */
export function mayCreateIdentity(state: InvitationState): boolean {
	return state.phase === 'creating';
}

export type Upstream = 'online' | 'offline' | 'unknown';

/** Copy for "Just the internet, please". Never claims the choice granted anything. */
export function internetCopy(upstream: Upstream): string {
	switch (upstream) {
		case 'online':
			return 'The internet is already available on this network. Nothing was changed by this choice.';
		case 'offline':
			return 'This network is local-only right now: there is no route to the internet. Nothing was changed by this choice, and local information below still works.';
		case 'unknown':
			return 'Nothing was changed by this choice. Whether this network reaches the internet is set by the network, not by this page.';
	}
}

export const dismissCopy =
	'Invitation dismissed. Nothing was created, and local information is still below.';

export type StorageVerdict = 'verified' | 'unverified';

export interface StorageProbe {
	userAgent: string;
	hasIndexedDb: boolean;
	writeReadOk: boolean;
	/** True when the page is a captive-portal mini browser (e.g. framed or sheet). */
	portalHint?: boolean;
}

const PORTAL_UA = /CaptiveNetworkSupport|wispr|NetworkExtension|captive/i;

/** Fails toward "unverified": only a passing write/read probe outside a portal hint counts. */
export function storageVerdict(probe: StorageProbe | undefined): StorageVerdict {
	if (!probe) return 'unverified';
	if (probe.portalHint || PORTAL_UA.test(probe.userAgent)) return 'unverified';
	return probe.hasIndexedDb && probe.writeReadOk ? 'verified' : 'unverified';
}

export const regularBrowserLimit =
	'This may be a temporary sign-in browser that does not keep keys. An identity made here could vanish when it closes.';

export const HUB_URL = 'http://hello.mesh/';

/** Stable local hub URLs: name first, then the node LAN gateway. No query, hash, or token. */
export function hubUrls(lanGateway?: string): string[] {
	const urls = [HUB_URL];
	if (lanGateway && /^\d{1,3}(\.\d{1,3}){3}$/.test(lanGateway)) urls.push(`http://${lanGateway}/`);
	return urls;
}

/** Regular-browser path: a bare local URL to type or scan. Never carries key material. */
export function regularBrowserPath(lanGateway?: string): string[] {
	return hubUrls(lanGateway);
}

/** Never accept a seed in the portal path. */
export const portalAsksForSeed = false;
