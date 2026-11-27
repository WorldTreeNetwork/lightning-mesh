import { guildSnapshot } from '$lib/guild/api';
import { groupsForHome, groupsFromSnapshot, LOCAL, type SavedGroup } from './groups';

const KEY = 'lightning-admin-group';

function storedId(): string {
	try {
		return localStorage.getItem(KEY) ?? '';
	} catch {
		return '';
	}
}

class GroupSelection {
	groups = $state<SavedGroup[]>([]);
	selectedId = $state('');

	constructor() {
		this.selectedId = storedId();
	}

	async load() {
		try {
			if (import.meta.env.DEV && typeof location !== 'undefined' && /\bdemo=1\b/.test(location.search)) {
				const { demoGroups } = await import('./demo');
				this.groups = demoGroups();
			} else {
				this.groups = groupsFromSnapshot(await guildSnapshot());
			}
		} catch {
			this.groups = [];
		}
		if (this.selectedId !== LOCAL.id && !groupsForHome(this.groups).some((group) => group.id === this.selectedId)) {
			this.select(LOCAL.id);
		}
	}

	select(id: string) {
		this.selectedId = id;
		try {
			localStorage.setItem(KEY, id);
		} catch {
			/* private mode */
		}
	}
}

export const groupSelection = new GroupSelection();
