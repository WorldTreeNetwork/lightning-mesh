import type { DirectoryService } from '$lib/directory/api';
import {
	isMiniApp,
	parseAppsResponse,
	type MiniAppManifest,
	type MiniAppRecord
} from './contract';

export interface ShelfTile {
	service: DirectoryService;
	record: MiniAppRecord | null;
}

export async function fetchAppsJson(fetchImpl: typeof fetch = fetch): Promise<unknown> {
	const res = await fetchImpl('/api/apps');
	if (!res.ok) {
		throw new Error(`GET /api/apps failed: ${res.status} ${res.statusText}`);
	}
	return res.json();
}

/** Directory is the tile set. `/api/apps` is decoration only. */
export function shelfTiles(
	services: DirectoryService[],
	appsJson: unknown | null
): ShelfTile[] {
	const decorated = appsJson === null ? [] : parseAppsResponse(appsJson, services);
	return services.filter(isMiniApp).map((service) => ({
		service,
		record:
			decorated.find(
				(row) =>
					row.service.name === service.name &&
					row.service.protocol === service.protocol &&
					row.service.ip === service.ip &&
					row.service.port === service.port
			) ?? null
	}));
}

export function tileManifest(tile: ShelfTile): MiniAppManifest | null {
	return tile.record?.manifest ?? null;
}

export function tileIsLinkOut(tile: ShelfTile): boolean {
	const manifest = tileManifest(tile);
	return manifest === null || manifest.embed === 'link';
}

export function bindCardMessages(
	target: {
		addEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
		removeEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
	},
	listener: (event: MessageEvent) => void
): () => void {
	target.addEventListener('message', listener);
	return () => target.removeEventListener('message', listener);
}
