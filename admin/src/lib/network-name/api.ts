import { invoke } from '@tauri-apps/api/core';

export interface ApplyReport {
	updated: string[];
	skipped: string[];
	halted: string | null;
	ok: boolean;
	log: string;
}

export async function applyNetworkName(name: string): Promise<ApplyReport> {
	return invoke<ApplyReport>('apply_network_name', { name });
}
