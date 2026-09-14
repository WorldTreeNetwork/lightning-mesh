import { describe, expect, it, vi } from 'vitest';
import type { DirectoryService } from '$lib/directory/api';
import {
	FRAME_ALLOW,
	FRAME_REFERRER,
	FRAME_SANDBOX,
	acceptBridgeMessage,
	appPath,
	canEmbed,
	entryOrigin,
	entryUrl,
	isMiniApp,
	parseAppsResponse,
	parseManifest,
	safeOpenUrl,
	sendBridgeMessage
} from './contract';

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

function bridgeEvent(data: unknown, origin = 'http://keyed.mesh:3000', source: unknown = frame) {
	return { data, origin, source };
}

const frame = {};

describe('App marker on service records', () => {
	it('Marked web service is a mini-app', () => {
		const svc = service();
		expect(isMiniApp(svc)).toBe(true);
		expect(appPath(svc)).toBe('/app');
		expect(entryUrl(svc)).toBe('http://keyed.mesh:3000/app');
		expect(entryOrigin(svc)).toBe('http://keyed.mesh:3000');
	});

	it('Unmarked service stays a plain service', () => {
		expect(isMiniApp(service({ txt: {} }))).toBe(false);
	});

	it('Bad path is not an app', () => {
		const svc = service({ txt: { app: 'v1', path: '//evil.example/x' } });
		expect(appPath(svc)).toBeUndefined();
		expect(isMiniApp(svc)).toBe(false);
	});

	it('Non-web protocol is not an app', () => {
		expect(isMiniApp(service({ protocol: 'ipp' }))).toBe(false);
	});
});

describe('App manifest', () => {
	const fixtures = import.meta.glob('./fixtures/manifests/*.json', {
		eager: true,
		import: 'default'
	}) as Record<string, { input: unknown; expect: unknown }>;

	for (const [path, fixture] of Object.entries(fixtures)) {
		it(`matches shared fixture ${path.split('/').at(-1)}`, () => {
			expect(parseManifest(fixture.input)).toEqual(fixture.expect);
		});
	}

	it('rejects an inlined icon over 64 KiB', () => {
		const payload = 'A'.repeat(Math.ceil((64 * 1024 + 1) / 3) * 4);
		expect(parseManifest({ v: 1, icon: `data:image/png;base64,${payload}` })).toBeNull();
	});

	it("Visitor's browser contacts no app host on load", () => {
		const fetchSpy = vi.fn();
		const directory = [service(), service({ name: 'notes' }), service({ name: 'chat' })];
		const response = {
			version: 1,
			apps: directory.map((svc) => ({ ...svc, manifest: { v: 1 }, url: 'http://evil.example' }))
		};
		expect(parseAppsResponse(response, directory)).toHaveLength(3);
		expect(fetchSpy).not.toHaveBeenCalled();
	});

	it('binds app metadata to the matching directory tuple and ignores server URL authority', () => {
		const svc = service();
		const parsed = parseAppsResponse(
			{
				version: 1,
				apps: [
					{
						name: svc.name,
						protocol: svc.protocol,
						ip: svc.ip,
						port: svc.port,
						manifest: { v: 1, embed: 'card' },
						url: 'http://hello.mesh/',
						origin: 'http://hello.mesh'
					}
				]
			},
			[svc]
		);
		expect(parsed).toEqual([
			{
				service: svc,
				manifest: { v: 1, embed: 'card' },
				fetchedAtUnixMs: 0,
				stale: false,
				error: null
			}
		]);
		expect(entryOrigin(parsed[0].service)).toBe('http://keyed.mesh:3000');
	});
});

describe('Sandboxed insertion', () => {
	it('Card opens on tap', () => {
		expect(canEmbed('http://keyed.mesh:3000', 'http://hello.mesh', 'card')).toBe(true);
		expect(FRAME_SANDBOX).toBe(
			'allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox'
		);
		expect(FRAME_ALLOW).toBe('');
		expect(FRAME_REFERRER).toBe('no-referrer');
	});

	it('No frame loads before the tap', () => {
		const fetchSpy = vi.fn();
		for (const origin of ['http://one.mesh', 'http://two.mesh', 'http://three.mesh']) {
			expect(canEmbed(origin, 'http://hello.mesh', 'card')).toBe(true);
		}
		expect(fetchSpy).not.toHaveBeenCalled();
	});

	it('Same-origin entry is refused as a card', () => {
		expect(canEmbed('http://10.42.7.1', 'http://10.42.7.1', 'card')).toBe(false);
	});

	it("Another node's gateway origin is refused as a card", () => {
		expect(canEmbed('http://10.42.7.1', 'http://hello.mesh', 'card')).toBe(false);
		expect(canEmbed('http://[fd00::1]', 'http://hello.mesh', 'card')).toBe(false);
	});

	it('Reserved name is refused as a card', () => {
		expect(canEmbed('http://id.mesh', 'http://hello.mesh', 'card')).toBe(false);
	});
});

describe('Bridge envelope', () => {
	it('Message from a foreign origin is ignored', () => {
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v1', type: 'resize', height: 400 }, 'http://other.mesh'),
				frame,
				'http://keyed.mesh:3000'
			)
		).toBeNull();
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v2', type: 'ready' }),
				frame,
				'http://keyed.mesh:3000'
			)
		).toBeNull();
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v1', type: 'surprise' }),
				frame,
				'http://keyed.mesh:3000'
			)
		).toBeNull();
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v1', type: 'ready' }, 'http://keyed.mesh:3000', {}),
				frame,
				'http://keyed.mesh:3000'
			)
		).toBeNull();
	});

	it('Frame navigated away gets nothing', () => {
		const navigatedFrame = { postMessage: vi.fn() };
		sendBridgeMessage(
			navigatedFrame,
			{ mesh: 'mini-app/v1', type: 'init', v: 1 },
			'http://keyed.mesh:3000'
		);
		expect(navigatedFrame.postMessage).toHaveBeenCalledWith(
			{ mesh: 'mini-app/v1', type: 'init', v: 1 },
			'http://keyed.mesh:3000'
		);
	});

	it('Resize is clamped', () => {
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v1', type: 'resize', height: 5000 }),
				frame,
				'http://keyed.mesh:3000'
			)
		).toEqual({ mesh: 'mini-app/v1', type: 'resize', height: 640 });
	});

	it('Non-finite height is ignored', () => {
		for (const height of [Number.NaN, Number.POSITIVE_INFINITY, '400']) {
			expect(
				acceptBridgeMessage(
					bridgeEvent({ mesh: 'mini-app/v1', type: 'resize', height }),
					frame,
					'http://keyed.mesh:3000'
				)
			).toBeNull();
		}
	});

	it('Oversize or unserializable message is ignored', () => {
		const oversize = { mesh: 'mini-app/v1', type: 'ready', padding: 'é'.repeat(8192) };
		expect(acceptBridgeMessage(bridgeEvent(oversize), frame, 'http://keyed.mesh:3000')).toBeNull();
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v1', type: 'ready', value: 1n }),
				frame,
				'http://keyed.mesh:3000'
			)
		).toBeNull();
	});

	it('Only web URLs open', () => {
		expect(
			acceptBridgeMessage(
				bridgeEvent({ mesh: 'mini-app/v1', type: 'open', url: 'javascript:alert(1)' }),
				frame,
				'http://keyed.mesh:3000'
			)
		).toBeNull();
		expect(safeOpenUrl('https://keyed.mesh/app')).toBe('https://keyed.mesh/app');
	});
});
