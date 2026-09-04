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
	selectedScope = $state<'all' | 'unique-local' | 'link-local'>('unique-local');

	get scopedNeighbors(): LinkLocalNeighbor[] {
		if (this.selectedScope === 'all') return this.neighbors;
		return this.neighbors.filter((n) => n.scope === this.selectedScope);
	}

	get visibleNeighbors(): LinkLocalNeighbor[] {
		if (this.selectedIface === 'all') return this.scopedNeighbors;
		return this.scopedNeighbors.filter((n) => n.iface === this.selectedIface);
	}

	get networkInterfaces(): LinkLocalInterface[] {
		return this.interfaces.filter((i) => !i.is_loopback);
	}

	neighborCount(name: string): number {
		return this.neighbors.filter((n) => {
			if (n.iface !== name) return false;
			if (this.selectedScope !== 'all' && n.scope !== this.selectedScope) return false;
			return true;
		}).length;
	}

	selectScope(scope: 'all' | 'unique-local' | 'link-local') {
		this.selectedScope = scope;
	}

	get uniqueLocalCount(): number {
		return this.neighbors.filter((n) => n.scope === 'unique-local').length;
	}

	get linkLocalCount(): number {
		return this.neighbors.filter((n) => n.scope === 'link-local').length;
	}

	visibleListText(): string {
		return this.visibleNeighbors
			.map((n) => `${n.base58}\t${n.address}\t${n.iface}`)
			.join('\n');
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
