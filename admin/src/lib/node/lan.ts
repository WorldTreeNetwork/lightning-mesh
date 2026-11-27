import type { Resource } from '$lib/discovery/resources';

export type LanTarget = { address: string; scopeId: number };

export const LAN_PATHS = ['/api/node', '/api/radio', '/api/directory', '/api/health'] as const;
export type LanPath = (typeof LAN_PATHS)[number];

type Row = { address: string; ifindex: number };

function bare(address: string): string {
	return address.split('%')[0].trim();
}

function isProbeable(address: string): boolean {
	const host = bare(address).toLowerCase();
	if (!host || host === '0.0.0.0' || host === '255.255.255.255' || host === '::' || host === '::1') {
		return false;
	}
	if (host.startsWith('127.') || host.startsWith('ff')) return false;
	const v4 = host.split('.');
	if (v4.length === 4) {
		if (v4.some((part) => !/^\d+$/.test(part))) return false;
		const first = Number(v4[0]);
		const last = Number(v4[3]);
		if (first >= 224 && first <= 239) return false;
		if (last === 255) return false;
		return v4.every((part) => Number(part) <= 255);
	}
	return host.includes(':');
}

function isMeshLan(address: string): boolean {
	const parts = bare(address).split('.');
	return parts.length === 4 && parts[0] === '10' && parts[1] === '42' && parts[3] === '1';
}

function isV4Gateway(address: string): boolean {
	const parts = bare(address).split('.');
	return parts.length === 4 && parts[3] === '1' && parts.every((part) => /^\d+$/.test(part));
}

/** LAN address already on the scan. Prefer the node's `10.42.*.1` gateway. */
export function lanTarget(rows: Row[]): LanTarget | null {
	const usable = rows.filter((row) => isProbeable(row.address));
	const mesh = usable.find((row) => isMeshLan(row.address));
	const gateway = usable.find((row) => isV4Gateway(row.address));
	const v4 = usable.find((row) => bare(row.address).split('.').length === 4);
	const pick = mesh ?? gateway ?? v4 ?? usable[0];
	if (!pick) return null;
	const address = bare(pick.address);
	const linkLocal = address.toLowerCase().startsWith('fe80:');
	return { address, scopeId: linkLocal ? pick.ifindex : 0 };
}

export function resourceForMesh(resources: Resource[], meshId: string): Resource | undefined {
	const key = meshId.trim().toLowerCase();
	if (!key) return undefined;
	return resources.find((resource) => resource.meshNodeId?.trim().toLowerCase() === key);
}

export function lanForMesh(resources: Resource[], meshId: string): LanTarget | null {
	const resource = resourceForMesh(resources, meshId);
	return resource ? lanTarget(resource.rows) : null;
}
