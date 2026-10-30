import type { DirectoryHttpsName, HttpsState } from '../directory/api';

const DAY_SECONDS = 86_400;

/** Mirrors mjolnir_mesh::https_status; all times are Unix seconds. */
export function httpsStatus(now: number, notAfter: number, lastRenewalFailed: boolean): HttpsState {
	if (!Number.isFinite(notAfter) || now >= notAfter) return 'expired';
	if (lastRenewalFailed) return 'failing';
	if (notAfter - now <= 10 * DAY_SECONDS) return 'due';
	return 'ok';
}

/** Recheck time locally so a stale directory cannot keep an expired link live. */
export function httpsNameDisplay(name: DirectoryHttpsName, now: number) {
	const state =
		name.state === 'expired'
			? 'expired'
			: httpsStatus(now, name.not_after, name.state === 'failing');
	const expired = state === 'expired';
	// Round up partial days: a live certificate never displays zero days left.
	const days = expired ? 0 : Math.ceil((name.not_after - now) / DAY_SECONDS);
	return {
		state,
		url: expired ? undefined : `https://${name.fqdn}`,
		message: expired
			? 'Connect a mesh gateway to the internet to renew HTTPS.'
			: `${days} days of HTTPS left offline`
	};
}
