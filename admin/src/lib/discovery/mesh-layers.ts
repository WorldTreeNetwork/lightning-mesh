export type MeshPath = { dst: string; nextHop: string };

/** Modified EUI-64 in a link-local address, back to the radio MAC. */
export function macFromIpv6(address: string): string | null {
	const bare = address.split('%')[0]?.trim().toLowerCase() ?? '';
	const bytes = expandIpv6(bare);
	if (!bytes) return null;
	const id = bytes.slice(8);
	if (id[3] !== 0xff || id[4] !== 0xfe) return null;
	const mac = [id[0] ^ 0x02, id[1], id[2], id[5], id[6], id[7]];
	return mac.map((byte) => byte.toString(16).padStart(2, '0')).join(':');
}

function expandIpv6(address: string): number[] | null {
	if (!address.includes(':')) return null;
	const pieces = address.split('::');
	if (pieces.length > 2) return null;
	const parse = (part: string) => (part ? part.split(':').filter((group) => group.length > 0) : []);
	const head = parse(pieces[0] ?? '');
	const tail = pieces.length === 2 ? parse(pieces[1] ?? '') : [];
	const missing = pieces.length === 2 ? 8 - head.length - tail.length : 0;
	if (missing < 0) return null;
	const groups = pieces.length === 2 ? [...head, ...Array(missing).fill('0'), ...tail] : head;
	if (groups.length !== 8) return null;
	const bytes: number[] = [];
	for (const group of groups) {
		if (!/^[0-9a-f]{1,4}$/.test(group)) return null;
		const value = Number.parseInt(group, 16);
		bytes.push((value >> 8) & 0xff, value & 0xff);
	}
	return bytes;
}

/**
 * Hops from the router this computer is attached to.
 * A station, or a path whose next hop is itself, is one hop.
 * A path whose next hop is some other radio is one past that radio.
 */
export function hopDistances(vantageMac: string, paths: MeshPath[], stations: string[]): Map<string, number> {
	const hop = new Map<string, number>();
	const vantage = vantageMac.trim().toLowerCase();
	if (!vantage) return hop;
	hop.set(vantage, 0);
	for (const mac of stations) {
		const key = mac.trim().toLowerCase();
		if (!key || key === vantage) continue;
		hop.set(key, Math.min(hop.get(key) ?? Number.POSITIVE_INFINITY, 1));
	}
	for (const path of paths) {
		const dst = path.dst.trim().toLowerCase();
		const next = path.nextHop.trim().toLowerCase();
		if (!dst || dst === vantage || dst !== next) continue;
		hop.set(dst, Math.min(hop.get(dst) ?? Number.POSITIVE_INFINITY, 1));
	}
	let changed = true;
	while (changed) {
		changed = false;
		for (const path of paths) {
			const dst = path.dst.trim().toLowerCase();
			const next = path.nextHop.trim().toLowerCase();
			if (!dst || !next || dst === next) continue;
			const via = hop.get(next);
			if (via === undefined) continue;
			const dist = via + 1;
			if (dist < (hop.get(dst) ?? Number.POSITIVE_INFINITY)) {
				hop.set(dst, dist);
				changed = true;
			}
		}
	}
	return hop;
}

export function formatHops(hops: number | null): string {
	if (hops === null) return '—';
	if (hops === 0) return 'here';
	if (hops === 1) return '1 hop';
	return `${hops} hops`;
}

/** The lightning gateway on the same IPv4 subnet as one of this computer's addresses. */
export function gatewayOnInterface(
	localAddrs: string[],
	candidates: { id: string; addresses: string[] }[]
): string | null {
	for (const addr of localAddrs) {
		const ip = addr.split('/')[0]?.trim() ?? '';
		const parts = ip.split('.');
		if (parts.length !== 4 || parts.some((part) => !/^\d+$/.test(part))) continue;
		const gateway = `${parts[0]}.${parts[1]}.${parts[2]}.1`;
		const found = candidates.find((candidate) =>
			candidate.addresses.some((item) => item.split('%')[0] === gateway)
		);
		if (found) return found.id;
	}
	return null;
}
