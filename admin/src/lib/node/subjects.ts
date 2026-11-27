import type { Resource } from '$lib/discovery/resources';
import { heardRouters, houseTiles, type HeardRouter, type SavedGroup } from '$lib/home/groups';
import { lanForMesh, type LanTarget } from './lan';

export type Subject = {
	id: string;
	name: string;
	heard: boolean;
	target: LanTarget | null;
};

/** Local is whoever the scan heard. A guild is its roster, heard and not heard. */
export function groupSubjects(
	local: boolean,
	resources: Resource[],
	group: SavedGroup,
	heard: HeardRouter[]
): Subject[] {
	const tiles = local
		? heardRouters(resources).map((router) => ({ id: router.id, name: router.name, heard: true }))
		: houseTiles(group, heard);
	return tiles.map((tile) => ({
		id: tile.id,
		name: tile.name,
		heard: tile.heard,
		target: tile.heard ? lanForMesh(resources, tile.id) : null
	}));
}
