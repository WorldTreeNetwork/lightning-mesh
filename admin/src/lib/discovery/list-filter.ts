/** Which neighbor states count as a live entry. `noarp` stays: gateways often have no ARP. */
const LIVE = new Set(['reachable', 'permanent', 'announced', 'local', 'delay', 'probe', 'noarp']);
const STALE = new Set(['stale']);
const FAILED = new Set(['failed', 'down', 'incomplete', 'none']);

export type StateBucket = 'live' | 'stale' | 'failed';

export type ListFilters = {
	live: boolean;
	stale: boolean;
	failed: boolean;
	lightning: boolean;
	gateway: boolean;
};

/** Stale and failed stay off until the operator asks for them. */
export const DEFAULT_FILTERS: ListFilters = {
	live: true,
	stale: false,
	failed: false,
	lightning: false,
	gateway: false
};

export type PresetId = 'all' | 'lightning' | 'gateways' | 'problems';

/** Named cuts. All is every live device. Lightning is live Lightning nodes. */
export const PRESETS: { id: PresetId; label: string; filters: ListFilters }[] = [
	{ id: 'all', label: 'All', filters: { ...DEFAULT_FILTERS } },
	{
		id: 'lightning',
		label: 'Lightning',
		filters: { live: true, stale: false, failed: false, lightning: true, gateway: false }
	},
	{
		id: 'gateways',
		label: 'Gateways',
		filters: { live: true, stale: false, failed: false, lightning: false, gateway: true }
	},
	{
		id: 'problems',
		label: 'Problems',
		filters: { live: false, stale: true, failed: true, lightning: false, gateway: false }
	}
];

export function matchingPreset(filters: ListFilters): PresetId | null {
	const found = PRESETS.find((preset) =>
		(Object.keys(preset.filters) as (keyof ListFilters)[]).every((key) => preset.filters[key] === filters[key])
	);
	return found?.id ?? null;
}

/** Empty means every interface. A plain click replaces the set. Ctrl toggles one name. */
export function nextIfaceSelection(current: string[], name: string, multi: boolean): string[] {
	if (!multi) {
		return current.length === 1 && current[0] === name ? [] : [name];
	}
	if (current.includes(name)) return current.filter((item) => item !== name);
	return [...current, name];
}

export function ifaceSelectionLabel(selected: string[]): string {
	if (selected.length === 0) return 'All interfaces';
	if (selected.length === 1) return selected[0];
	return `${selected.length} interfaces`;
}

export type Filterable = {
	state: string;
	meshName: string | null;
	router: boolean;
	lightning?: boolean;
};

export function isLightning(resource: { meshName: string | null; lightning?: boolean }): boolean {
	return Boolean(resource.lightning || resource.meshName?.trim());
}

export function stateBucket(state: string): StateBucket {
	if (STALE.has(state)) return 'stale';
	if (FAILED.has(state)) return 'failed';
	if (LIVE.has(state)) return 'live';
	return 'live';
}

/** State buckets combine with OR. Lightning and Gateway narrow with AND. */
export function resourceMatches(resource: Filterable, filters: ListFilters): boolean {
	const bucket = stateBucket(resource.state);
	if (!filters[bucket]) return false;
	if (filters.lightning && !isLightning(resource)) return false;
	if (filters.gateway && !resource.router) return false;
	return true;
}
