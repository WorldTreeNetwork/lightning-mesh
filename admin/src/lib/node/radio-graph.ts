import type { RadioPath, RadioStation } from './documents';

export type GraphInput = {
	id: string;
	name: string;
	quiet: boolean;
	mac?: string | null;
	stations?: RadioStation[];
	mpaths?: RadioPath[];
};

export type GraphNode = {
	id: string;
	name: string;
	quiet: boolean;
	x: number;
	y: number;
};

export type GraphEdge = {
	from: string;
	to: string;
	label: string;
	quiet: boolean;
};

export type RadioGraph = {
	nodes: GraphNode[];
	edges: GraphEdge[];
	paths: string[];
};

function macKey(mac: string): string {
	return mac.trim().toLowerCase();
}

function pairKey(a: string, b: string): string {
	return a < b ? `${a}|${b}` : `${b}|${a}`;
}

/** Circle of radios. Station edges carry signal. A path is a quieter edge only when no station edge exists. */
export function layoutRadio(input: GraphInput[]): RadioGraph {
	const nodes: GraphNode[] = [];
	const byMac = new Map<string, string>();
	for (const node of input) {
		nodes.push({ id: node.id, name: node.name, quiet: node.quiet, x: 0, y: 0 });
		if (node.mac) byMac.set(macKey(node.mac), node.id);
	}

	const station = new Map<string, GraphEdge>();
	for (const node of input) {
		for (const peer of node.stations ?? []) {
			const key = macKey(peer.mac);
			let other = byMac.get(key);
			if (!other) {
				other = `mac:${key}`;
				if (!nodes.some((item) => item.id === other)) {
					nodes.push({ id: other, name: peer.mac, quiet: false, x: 0, y: 0 });
				}
				byMac.set(key, other);
			}
			if (other === node.id) continue;
			const label = peer.signal_dbm === null ? 'station' : `${peer.signal_dbm} dBm`;
			const edgeKey = pairKey(node.id, other);
			const existing = station.get(edgeKey);
			if (!existing || (peer.signal_dbm !== null && !existing.label.includes('dBm'))) {
				station.set(edgeKey, { from: node.id, to: other, label, quiet: false });
			}
		}
	}

	const paths: string[] = [];
	const quiet = new Map<string, GraphEdge>();
	for (const node of input) {
		for (const path of node.mpaths ?? []) {
			const metric = path.metric === null ? '' : ` metric ${path.metric}`;
			paths.push(`${node.name}: ${path.dst} via ${path.next_hop}${metric}`);
			const dest = byMac.get(macKey(path.dst));
			if (!dest || dest === node.id) continue;
			const edgeKey = pairKey(node.id, dest);
			if (station.has(edgeKey) || quiet.has(edgeKey)) continue;
			quiet.set(edgeKey, { from: node.id, to: dest, label: 'path', quiet: true });
		}
	}

	nodes.sort((a, b) => Number(a.quiet) - Number(b.quiet) || a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
	const count = nodes.length;
	const cx = 320;
	const cy = 190;
	const radius = count <= 1 ? 0 : 148;
	nodes.forEach((node, index) => {
		const angle = -Math.PI / 2 + (2 * Math.PI * index) / Math.max(count, 1);
		node.x = cx + radius * Math.cos(angle);
		node.y = cy + radius * Math.sin(angle);
	});

	return { nodes, edges: [...station.values(), ...quiet.values()], paths };
}
