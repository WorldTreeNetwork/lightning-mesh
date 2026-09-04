import { invoke } from '@tauri-apps/api/core';

export interface LinkLocalInterface {
	name: string;
	index: number;
	mac: string | null;
	is_up: boolean;
	is_loopback: boolean;
	link_local: string[];
	unique_local: string[];
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
