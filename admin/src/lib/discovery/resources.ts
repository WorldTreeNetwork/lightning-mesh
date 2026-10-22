import type { LinkLocalNeighbor } from './api';

export type Resource = {
	id: string;
	/** Folded by MAC or hostname without an endpoint id. */
	guess: boolean;
	rows: LinkLocalNeighbor[];
	primary: LinkLocalNeighbor;
	state: string;
	mac: string | null;
	iroh: string | null;
	hostname: string | null;
	meshName: string | null;
	meshNodeId: string | null;
	meshSubnet: string | null;
	meshBackhaul: string | null;
	router: boolean;
	/** Gateways seen on the same links as this resource. */
	via: string[];
	local: boolean;
};

const STATE_RANK = ['reachable', 'permanent', 'announced', 'local', 'delay', 'probe', 'stale', 'incomplete', 'failed', 'down'];

function stateRank(state: string): number {
	const i = STATE_RANK.indexOf(state);
	return i === -1 ? STATE_RANK.length : i;
}

function preferAddress(a: LinkLocalNeighbor, b: LinkLocalNeighbor): number {
	const rank = (n: LinkLocalNeighbor) => (n.scope === 'ipv4' ? 0 : n.scope === 'unique-local' ? 1 : 2);
	return rank(a) - rank(b) || stateRank(a.state) - stateRank(b.state) || a.address.localeCompare(b.address);
}

function hostOf(row: LinkLocalNeighbor): string {
	return (row.hostname ?? '').trim().toLowerCase();
}

class SetId {
	parent: number[] = [];
	find(i: number): number {
		if (this.parent[i] !== i) this.parent[i] = this.find(this.parent[i]);
		return this.parent[i];
	}
	union(a: number, b: number) {
		const ra = this.find(a);
		const rb = this.find(b);
		if (ra !== rb) this.parent[rb] = ra;
	}
	add(): number {
		const id = this.parent.length;
		this.parent.push(id);
		return id;
	}
}

export function bestState(rows: LinkLocalNeighbor[]): string {
	return [...rows].sort((a, b) => stateRank(a.state) - stateRank(b.state))[0]?.state ?? '';
}

function labelOf(
	resource: Pick<Resource, 'hostname' | 'meshName' | 'router' | 'local' | 'primary'>
): string {
	if (resource.local) return 'This computer';
	if (resource.meshName) return resource.meshName;
	if (resource.hostname) return resource.hostname.replace(/\.local$/i, '');
	if (resource.router) return 'Gateway';
	return resource.primary.address;
}

/** One machine: shared MAC, shared mDNS name, or shared endpoint id. */
export function groupResources(rows: LinkLocalNeighbor[]): Resource[] {
	const sets = new SetId();
	const index = rows.map(() => sets.add());
	const byMac = new Map<string, number>();
	const byHost = new Map<string, number>();
	const byIroh = new Map<string, number>();
	rows.forEach((row, i) => {
		if (row.kind === 'local') {
			const prev = byHost.get('__local__');
			if (prev === undefined) byHost.set('__local__', i);
			else sets.union(prev, i);
			return;
		}
		const mac = row.mac?.toLowerCase();
		if (mac) {
			const prev = byMac.get(mac);
			if (prev === undefined) byMac.set(mac, i);
			else sets.union(prev, i);
		}
		const host = hostOf(row);
		if (host) {
			const prev = byHost.get(host);
			if (prev === undefined) byHost.set(host, i);
			else sets.union(prev, i);
		}
		const mesh = row.mesh_node_id?.trim();
		if (mesh) {
			const prev = byIroh.get(`mesh:${mesh}`);
			if (prev === undefined) byIroh.set(`mesh:${mesh}`, i);
			else sets.union(prev, i);
		}
		const iroh = row.iroh?.trim();
		if (iroh) {
			const prev = byIroh.get(iroh);
			if (prev === undefined) byIroh.set(iroh, i);
			else sets.union(prev, i);
		}
	});

	const buckets = new Map<number, LinkLocalNeighbor[]>();
	rows.forEach((row, i) => {
		const root = sets.find(index[i]);
		const list = buckets.get(root);
		if (list) list.push(row);
		else buckets.set(root, [row]);
	});

	const resources: Resource[] = [];
	for (const grouped of buckets.values()) {
		const sorted = [...grouped].sort(preferAddress);
		const local = sorted.every((row) => row.kind === 'local');
		const macs = new Set(sorted.map((row) => row.mac?.toLowerCase()).filter((m): m is string => Boolean(m)));
		const hostname = sorted.find((row) => hostOf(row))?.hostname ?? null;
		const mesh = sorted.find((row) => row.mesh_name?.trim());
		const iroh = sorted.find((row) => row.iroh?.trim())?.iroh ?? null;
		const router = sorted.some((row) => row.router);
		const joinedByMac = macs.size === 1 && sorted.length > 1 && !iroh && !hostname;
		resources.push({
			id: mesh?.mesh_node_id
				? `mesh:${mesh.mesh_node_id}`
				: iroh
					? `iroh:${iroh}`
					: hostname
						? `host:${hostname.toLowerCase()}`
						: macs.size === 1
							? `mac:${[...macs][0]}`
							: `addr:${sorted[0].ifindex}:${sorted[0].address}`,
			guess: joinedByMac,
			rows: sorted,
			primary: sorted[0],
			state: bestState(sorted),
			mac: macs.size === 1 ? sorted.find((row) => row.mac)?.mac ?? null : null,
			iroh,
			hostname,
			meshName: mesh?.mesh_name ?? null,
			meshNodeId: mesh?.mesh_node_id ?? null,
			meshSubnet: mesh?.mesh_subnet ?? null,
			meshBackhaul: mesh?.mesh_backhaul ?? null,
			router,
			via: [],
			local
		});
	}

	const gateways = resources.filter((resource) => resource.router);
	for (const resource of resources) {
		if (resource.router || resource.local) continue;
		const ifaces = new Set(resource.rows.map((row) => row.ifindex));
		resource.via = gateways
			.filter((gateway) => gateway.rows.some((row) => ifaces.has(row.ifindex)))
			.map((gateway) => labelOf(gateway));
	}
	return resources;
}

export function resourceLabel(resource: Resource): string {
	return labelOf(resource);
}
