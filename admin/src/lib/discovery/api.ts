import { invoke } from '@tauri-apps/api/core';

export interface LinkLocalInterface {
	name: string;
	index: number;
	mac: string | null;
	is_up: boolean;
	is_loopback: boolean;
	link_local: string[];
}

export interface LinkLocalNeighbor {
	iface: string;
	ifindex: number;
	address: string;
	scoped: string;
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

export function scanLinkLocal(): Promise<ScanResult> {
	return invoke<ScanResult>('scan_link_local');
}
