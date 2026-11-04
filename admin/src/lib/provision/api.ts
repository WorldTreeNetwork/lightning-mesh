import { invoke } from '@tauri-apps/api/core';

export interface ProvisionReport {
	ok: boolean;
	log: string;
	nodeId: string;
	lamp: string;
}

export function provisionStockNode(host: string, name: string, group: string): Promise<ProvisionReport> {
	return invoke<ProvisionReport>('provision_stock_node', { host, name, group });
}

export function provisionNodeHealth(host: string): Promise<ProvisionReport> {
	return invoke<ProvisionReport>('provision_node_health', { host });
}
