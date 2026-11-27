import type { GuildSnapshot } from '$lib/guild/api';

export type HeardRouter = { id: string; name: string };
export type AssignedRouter = { id: string; name: string };
export type HouseTile = { id: string; name: string; heard: boolean };
export type Membership = 'admin' | 'member' | 'absent' | 'no-seed';

export type SavedGroup = {
	id: string;
	name: string;
	assigned: AssignedRouter[];
	you?: Membership;
	workplace?: boolean;
};

/** Home selection for every node this computer can see. Not a guild. */
export const LOCAL: SavedGroup = { id: 'local', name: 'Local', assigned: [] };

export type LocalTile = {
	id: string;
	name: string;
	detail: string;
	href: string;
};

export type SeenNode = {
	id: string;
	local: boolean;
	state: string;
	meshName: string | null;
	meshNodeId: string | null;
	hostname: string | null;
	router: boolean;
	primary: { address: string };
	houseUla?: string | null;
	meshBackhaul?: string | null;
};

export function needsAdmin(you: Membership | undefined): boolean {
	return you === 'member' || you === 'absent' || you === 'no-seed';
}

export type GroupCount = {
	active: number;
	total: number | null;
	heard: HeardRouter[];
};

const UNHEARD = new Set(['failed', 'down']);

/** Guilds worth choosing on Home. The empty Admin guild is the operator, not a network. */
export function groupsForHome(groups: SavedGroup[]): SavedGroup[] {
	return groups.filter((group) => group.workplace || group.name.trim().toLowerCase() !== 'admin');
}

function ignoredAddress(address: string): boolean {
	const bare = address.split('%')[0].trim().toLowerCase();
	if (bare === '0.0.0.0' || bare === '255.255.255.255' || bare.startsWith('ff')) return true;
	const parts = bare.split('.');
	if (parts.length !== 4 || parts.some((part) => !/^\d+$/.test(part))) return false;
	const first = Number(parts[0]);
	const last = Number(parts[3]);
	return (first >= 224 && first <= 239) || last === 255;
}

function seenName(node: SeenNode): string {
	const mesh = node.meshName?.trim();
	if (mesh) return mesh;
	const host = node.hostname?.trim();
	if (host) return host.replace(/\.local$/i, '');
	if (node.router) return 'Gateway';
	return node.primary.address;
}

/** Nodes the last scan can see, other than this computer. */
export function localTiles(nodes: SeenNode[]): LocalTile[] {
	const tiles: LocalTile[] = [];
	for (const node of nodes) {
		if (node.local || UNHEARD.has(node.state) || ignoredAddress(node.primary.address)) continue;
		const meshId = node.meshNodeId?.trim() ?? '';
		const name = seenName(node);
		if (!name) continue;
		tiles.push({
			id: meshId || node.id,
			name,
			detail: node.houseUla?.trim() || node.meshBackhaul?.trim() || node.primary.address,
			href: meshId ? `/router/${meshId}` : `/host/${encodeURIComponent(node.id)}`
		});
	}
	tiles.sort((a, b) => a.name.localeCompare(b.name) || a.detail.localeCompare(b.detail));
	return tiles;
}

export function formatCount(count: GroupCount): string {
	return count.total === null ? `${count.active}/–` : `${count.active}/${count.total}`;
}

/** Local is a head count. A guild is heard over assigned. */
export function selectionCount(group: SavedGroup, count: GroupCount): string {
	return group.id === LOCAL.id ? String(count.active) : formatCount(count);
}

export function heardRouters(
	resources: { meshName: string | null; meshNodeId: string | null; local: boolean; state: string }[]
): HeardRouter[] {
	const seen = new Set<string>();
	const heard: HeardRouter[] = [];
	for (const resource of resources) {
		const name = resource.meshName?.trim();
		const id = resource.meshNodeId?.trim();
		if (!name || !id || resource.local || UNHEARD.has(resource.state)) continue;
		const key = id.toLowerCase();
		if (seen.has(key)) continue;
		seen.add(key);
		heard.push({ id, name });
	}
	heard.sort((a, b) => a.name.localeCompare(b.name));
	return heard;
}

export function groupsFromSnapshot(snapshot: GuildSnapshot): SavedGroup[] {
	return snapshot.guilds
		.filter((guild) => guild.profile !== 'access-node')
		.sort((a, b) => Number(b.workplace) - Number(a.workplace))
		.map((guild) => ({
			id: guild.keyspaceId || guild.id,
			name: guild.name?.trim() || 'Group',
			you: guild.you,
			workplace: guild.workplace,
			assigned: guild.nodes.map((node) => ({
				id: node.nodeId,
				name: node.name?.trim() || 'Router'
			}))
		}));
}

/** No roster yet: the count is a placeholder and every heard router is listed. */
export function countGroup(group: SavedGroup, heard: HeardRouter[]): GroupCount {
	if (group.assigned.length === 0) {
		return { active: heard.length, total: null, heard };
	}
	const assigned = new Set(group.assigned.map((node) => node.id.toLowerCase()));
	const matched = heard.filter((router) => assigned.has(router.id.toLowerCase()));
	return { active: matched.length, total: group.assigned.length, heard: matched };
}

/** The house, in roster order. A group with no roster shows whoever the scan heard. */
export function houseTiles(group: SavedGroup, heard: HeardRouter[]): HouseTile[] {
	if (group.assigned.length === 0) {
		return heard.map((router) => ({ id: router.id, name: router.name, heard: true }));
	}
	const heardIds = new Set(heard.map((router) => router.id.toLowerCase()));
	return group.assigned.map((node) => ({
		id: node.id,
		name: node.name,
		heard: heardIds.has(node.id.toLowerCase())
	}));
}

export function routerTitle(id: string, group: SavedGroup, heard: HeardRouter[]): string {
	const key = id.trim().toLowerCase();
	if (!key) return 'Router';
	const assigned = group.assigned.find((node) => node.id.toLowerCase() === key);
	if (assigned?.name.trim()) return assigned.name.trim();
	const found = heard.find((router) => router.id.toLowerCase() === key);
	if (found?.name.trim()) return found.name.trim();
	return id;
}

export function present(
	groups: SavedGroup[],
	selectedId: string,
	heard: HeardRouter[],
	seen: HeardRouter[] = []
) {
	const list = [LOCAL, ...groupsForHome(groups)];
	const selected = list.find((group) => group.id === selectedId) ?? LOCAL;
	const countFor = (group: SavedGroup): GroupCount =>
		group.id === LOCAL.id
			? { active: seen.length, total: null, heard: seen }
			: countGroup(group, heard);
	return {
		list: list.map((group) => ({ group, count: countFor(group) })),
		selected,
		count: countFor(selected)
	};
}
