export type NodeDocument = {
	name?: string;
	node_id?: string;
	subnet?: string;
	backhaul_addr?: string;
	house_ula?: string | null;
	link_local_lan?: string;
	link_local_mesh?: string;
	ula?: string;
};

export type RadioStation = {
	mac: string;
	signal_dbm: number | null;
	expected_throughput_mbps: number | null;
	inactive_ms: number | null;
};

export type RadioPath = {
	dst: string;
	next_hop: string;
	metric: number | null;
};

export type RadioDocument = {
	backhaul_addr: string | null;
	mesh_if: string | null;
	mesh_mac: string | null;
	channel: number | null;
	freq_mhz: number | null;
	stations: RadioStation[];
	mpaths: RadioPath[];
};

export type DirectoryService = {
	name: string;
	ip: string;
	port: number;
	protocol: string;
	txt: Record<string, string>;
};

export type DirectoryDocument = {
	services: DirectoryService[];
};

function record(value: unknown): Record<string, unknown> | null {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
	return value as Record<string, unknown>;
}

function text(value: unknown): string | null {
	return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function num(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

export function asNodeDocument(value: unknown): NodeDocument {
	const body = record(value);
	if (!body) throw new Error('The node document is not an object.');
	return {
		name: text(body.name) ?? undefined,
		node_id: text(body.node_id) ?? undefined,
		subnet: text(body.subnet) ?? undefined,
		backhaul_addr: text(body.backhaul_addr) ?? undefined,
		house_ula: text(body.house_ula),
		link_local_lan: text(body.link_local_lan) ?? undefined,
		link_local_mesh: text(body.link_local_mesh) ?? undefined,
		ula: text(body.ula) ?? undefined
	};
}

export function asRadioDocument(value: unknown): RadioDocument {
	const body = record(value);
	if (!body) throw new Error('The radio document is not an object.');
	const stations = Array.isArray(body.stations) ? body.stations : [];
	const mpaths = Array.isArray(body.mpaths) ? body.mpaths : [];
	return {
		backhaul_addr: text(body.backhaul_addr),
		mesh_if: text(body.mesh_if),
		mesh_mac: text(body.mesh_mac),
		channel: num(body.channel),
		freq_mhz: num(body.freq_mhz),
		stations: stations.flatMap((row) => {
			const item = record(row);
			const mac = item ? text(item.mac) : null;
			if (!item || !mac) return [];
			return [
				{
					mac,
					signal_dbm: num(item.signal_dbm),
					expected_throughput_mbps: num(item.expected_throughput_mbps),
					inactive_ms: num(item.inactive_ms)
				}
			];
		}),
		mpaths: mpaths.flatMap((row) => {
			const item = record(row);
			const dst = item ? text(item.dst) : null;
			const next = item ? text(item.next_hop) : null;
			if (!item || !dst || !next) return [];
			return [{ dst, next_hop: next, metric: num(item.metric) }];
		})
	};
}

export function asDirectoryDocument(value: unknown): DirectoryDocument {
	const body = record(value);
	if (!body) throw new Error('The directory document is not an object.');
	const services = Array.isArray(body.services) ? body.services : [];
	return {
		services: services.flatMap((row) => {
			const item = record(row);
			const name = item ? text(item.name) : null;
			const ip = item ? text(item.ip) : null;
			const port = item ? num(item.port) : null;
			if (!item || !name || !ip || port === null) return [];
			const txt = record(item.txt);
			const entries: Record<string, string> = {};
			if (txt) {
				for (const [key, raw] of Object.entries(txt)) {
					if (typeof raw === 'string') entries[key] = raw;
				}
			}
			return [{ name, ip, port, protocol: text(item.protocol) ?? '', txt: entries }];
		})
	};
}

export type ServiceRow = DirectoryService & { reportedBy: string[] };

export function mergeServices(reports: { node: string; services: DirectoryService[] }[]): ServiceRow[] {
	const rows = new Map<string, ServiceRow>();
	for (const report of reports) {
		for (const service of report.services) {
			const key = `${service.name}\n${service.ip}\n${service.port}`;
			const existing = rows.get(key);
			if (existing) {
				if (!existing.reportedBy.includes(report.node)) existing.reportedBy.push(report.node);
				continue;
			}
			rows.set(key, { ...service, reportedBy: [report.node] });
		}
	}
	return [...rows.values()].sort(
		(a, b) => a.name.localeCompare(b.name) || a.ip.localeCompare(b.ip) || a.port - b.port
	);
}
