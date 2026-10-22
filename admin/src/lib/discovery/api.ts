import { invoke } from '@tauri-apps/api/core';

export interface LinkLocalInterface {
	name: string;
	index: number;
	mac: string | null;
	is_up: boolean;
	is_loopback: boolean;
	link_local: string[];
	unique_local: string[];
	ipv4?: string[];
}

export interface LinkLocalNeighbor {
	iface: string;
	ifindex: number;
	address: string;
	scoped: string;
	/** Bitcoin-alphabet base58 of the 16 IPv6 octets. */
	base58: string;
	/** `unique-local` (`fc00::/7`) or `link-local` (`fe80::/10`). */
	scope: 'unique-local' | 'link-local' | string;
	mac: string | null;
	state: string;
	kind: 'local' | 'neighbor' | string;
	source: string;
	/** Kernel marked this neighbor as a router on the link. */
	router?: boolean;
	/** mDNS hostname, such as `edgebox.local`. */
	hostname?: string | null;
	/** Present when `GET /api/node` on this address answered as a Lightning node. */
	mesh_name?: string | null;
	mesh_node_id?: string | null;
	mesh_subnet?: string | null;
	mesh_backhaul?: string | null;
	/** Iroh endpoint id, when this host publishes mesh resources. */
	iroh?: string | null;
}

export interface ScanResult {
	interfaces: LinkLocalInterface[];
	neighbors: LinkLocalNeighbor[];
	probed: boolean;
	probe_error: string | null;
}

export async function scanLinkLocal(): Promise<ScanResult> {
	if (import.meta.env.DEV && typeof location !== 'undefined' && /\bdemo=1\b/.test(location.search)) {
		const { demoScan } = await import('./fixture');
		return demoScan();
	}
	return invoke<ScanResult>('scan_link_local');
}
