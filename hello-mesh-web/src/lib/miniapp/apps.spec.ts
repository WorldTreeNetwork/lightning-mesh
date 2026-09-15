import { describe, expect, it } from 'vitest';
import type { DirectoryService } from '$lib/directory/api';
import { bindCardMessages, shelfTiles, tileIsLinkOut, tileManifest } from './apps';

function service(overrides: Partial<DirectoryService> = {}): DirectoryService {
	return {
		name: 'keyed',
		protocol: 'http',
		ip: '10.42.7.20',
		port: 3000,
		txt: { app: 'v1', path: '/app' },
		...overrides
	};
}

describe('shelfTiles', () => {
	it('uses the directory as the tile set when /api/apps is missing', () => {
		const unmarked = service({ name: 'wiki', txt: undefined });
		const tiles = shelfTiles([service(), unmarked], null);
		expect(tiles).toHaveLength(1);
		expect(tiles[0].service.name).toBe('keyed');
		expect(tileIsLinkOut(tiles[0])).toBe(true);
	});

	it('decorates from /api/apps without dropping unmatched mini-apps', () => {
		const other = service({ name: 'cam', port: 8080 });
		const tiles = shelfTiles(
			[service(), other],
			{
				version: 1,
				apps: [
					{
						name: 'keyed',
						protocol: 'http',
						ip: '10.42.7.20',
						port: 3000,
						manifest: { v: 1, name: 'Keyed', embed: 'card', height: 320 },
						stale: false
					}
				]
			}
		);
		expect(tiles).toHaveLength(2);
		expect(tileManifest(tiles[0])?.name).toBe('Keyed');
		expect(tileIsLinkOut(tiles[0])).toBe(false);
		expect(tileIsLinkOut(tiles[1])).toBe(true);
	});

	it('stale with a kept manifest still honours embed', () => {
		const tiles = shelfTiles(
			[service()],
			{
				version: 1,
				apps: [
					{
						name: 'keyed',
						protocol: 'http',
						ip: '10.42.7.20',
						port: 3000,
						manifest: { v: 1, embed: 'card' },
						stale: true
					}
				]
			}
		);
		expect(tileIsLinkOut(tiles[0])).toBe(false);
		expect(tiles[0].record?.stale).toBe(true);
	});

	it('bindCardMessages teardown stops delivery', () => {
		const delivered: MessageEvent[] = [];
		const listeners = new Set<(event: MessageEvent) => void>();
		const target = {
			addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
				listeners.add(listener);
			},
			removeEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
				listeners.delete(listener);
			}
		};
		const stop = bindCardMessages(target, (event) => delivered.push(event));
		for (const listener of listeners) listener({ data: 1 } as MessageEvent);
		expect(delivered).toHaveLength(1);
		stop();
		for (const listener of listeners) listener({ data: 2 } as MessageEvent);
		expect(delivered).toHaveLength(1);
	});

	it('null manifest is link-out even if stale', () => {
		const tiles = shelfTiles(
			[service()],
			{
				version: 1,
				apps: [
					{
						name: 'keyed',
						protocol: 'http',
						ip: '10.42.7.20',
						port: 3000,
						manifest: null,
						stale: true
					}
				]
			}
		);
		expect(tileIsLinkOut(tiles[0])).toBe(true);
	});
});
