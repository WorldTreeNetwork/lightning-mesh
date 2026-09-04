import { applyNetworkName, type ApplyReport } from './api';

/** Operator copy: the client AP SSID. Never "guild". */
export const NETWORK_NAME_LABEL = 'Network name';

export class NetworkNameStore {
	name = $state('');
	applying = $state(false);
	error = $state<string | null>(null);
	report = $state<ApplyReport | null>(null);

	get canApply(): boolean {
		return this.name.trim().length > 0 && !this.applying;
	}

	async apply() {
		const name = this.name.trim();
		if (!name || this.applying) return;
		this.applying = true;
		this.error = null;
		this.report = null;
		try {
			this.report = await applyNetworkName(name);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		} finally {
			this.applying = false;
		}
	}
}

export const networkName = new NetworkNameStore();
