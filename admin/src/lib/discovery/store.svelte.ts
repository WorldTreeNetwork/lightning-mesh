import { scanLinkLocal, type LinkLocalInterface, type LinkLocalNeighbor } from './api';

export class DiscoveryStore {
	interfaces = $state<LinkLocalInterface[]>([]);
	neighbors = $state<LinkLocalNeighbor[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);
	probeError = $state<string | null>(null);
	probed = $state(false);
	lastScan = $state<number | null>(null);
	selectedIface = $state<string | 'all'>('all');

	get visibleNeighbors(): LinkLocalNeighbor[] {
		if (this.selectedIface === 'all') return this.neighbors;
		return this.neighbors.filter((n) => n.iface === this.selectedIface);
	}

	get networkInterfaces(): LinkLocalInterface[] {
		return this.interfaces.filter((i) => !i.is_loopback);
	}

	neighborCount(name: string): number {
		return this.neighbors.filter((n) => n.iface === name).length;
	}

	selectIface(name: string | 'all') {
		this.selectedIface = name;
	}

	async scan() {
		this.loading = true;
		this.error = null;
		try {
			const result = await scanLinkLocal();
			this.interfaces = result.interfaces;
			this.neighbors = result.neighbors;
			this.probed = result.probed;
			this.probeError = result.probe_error;
			this.lastScan = Date.now();
			if (
				this.selectedIface !== 'all' &&
				!this.interfaces.some((i) => i.name === this.selectedIface)
			) {
				this.selectedIface = 'all';
			}
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		} finally {
			this.loading = false;
		}
	}
}

export const discovery = new DiscoveryStore();
