import type { DirectoryService } from '$lib/directory/api';

export const FRAME_SANDBOX =
	'allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox';
export const FRAME_ALLOW = '';
export const FRAME_REFERRER = 'no-referrer';

const BRIDGE_VERSION = 'mini-app/v1';
const MAX_BRIDGE_BYTES = 16 * 1024;
const MAX_ICON_BYTES = 64 * 1024;
const MDNS_SERVICE_NAME = /\._[a-z]+\._[a-z]+$/i;
const RESERVED_KEY_HOSTS = new Set(['hello.mesh', 'id.mesh']);

export interface MiniAppManifest {
	v: 1;
	name?: string;
	description?: string;
	icon?: string;
	embed: 'card' | 'link';
	height?: number;
}

export type AppToHostMessage =
	| { mesh: typeof BRIDGE_VERSION; type: 'ready' }
	| { mesh: typeof BRIDGE_VERSION; type: 'resize'; height: number }
	| { mesh: typeof BRIDGE_VERSION; type: 'open'; url: string };

export type HostToAppMessage = { mesh: typeof BRIDGE_VERSION; type: 'init'; v: 1 };
export type BridgeEnvelope = AppToHostMessage | HostToAppMessage;

export interface BridgeMessageEvent {
	source: unknown;
	origin: string;
	data: unknown;
}

export interface MiniAppRecord {
	service: DirectoryService;
	manifest: MiniAppManifest | null;
	fetchedAtUnixMs: number;
	stale: boolean;
	error: string | null;
}

function appHost(svc: DirectoryService): string {
	if (MDNS_SERVICE_NAME.test(svc.name)) {
		return svc.ip.includes(':') && !svc.ip.startsWith('[') ? `[${svc.ip}]` : svc.ip;
	}
	return `${svc.name}.mesh`;
}

function recordOrigin(svc: DirectoryService): string | undefined {
	const protocol = svc.protocol.toLowerCase();
	if (protocol !== 'http' && protocol !== 'https') return undefined;
	const defaultPort = protocol === 'https' ? 443 : 80;
	const port = svc.port && svc.port !== defaultPort ? `:${svc.port}` : '';
	try {
		return new URL(`${protocol}://${appHost(svc)}${port}`).origin;
	} catch {
		return undefined;
	}
}

export function appPath(svc: DirectoryService): string | undefined {
	const path = svc.txt?.path ?? '/';
	if (
		!path.startsWith('/') ||
		path.includes('//') ||
		path.includes('\\') ||
		/[\u0000-\u001f\u007f]/.test(path) ||
		/(?:^|\/)[a-z][a-z\d+.-]*:/i.test(path)
	) {
		return undefined;
	}
	return path;
}

export function entryUrl(svc: DirectoryService): string | undefined {
	const origin = recordOrigin(svc);
	const path = appPath(svc);
	if (!origin || !path) return undefined;
	try {
		const entry = new URL(path, `${origin}/`);
		return entry.origin === origin ? entry.href : undefined;
	} catch {
		return undefined;
	}
}

export function entryOrigin(svc: DirectoryService): string | undefined {
	const entry = entryUrl(svc);
	return entry ? new URL(entry).origin : undefined;
}

export function isMiniApp(svc: DirectoryService): boolean {
	return (
		(svc.protocol.toLowerCase() === 'http' || svc.protocol.toLowerCase() === 'https') &&
		svc.txt?.app === 'v1' &&
		entryUrl(svc) !== undefined
	);
}

export function clampHeight(value: number): number {
	if (!Number.isFinite(value)) return 120;
	return Math.min(640, Math.max(120, value));
}

function truncate(value: string, limit: number): string {
	return [...value].slice(0, limit).join('');
}

function validDataImage(uri: string): boolean {
	if (new TextEncoder().encode(uri).byteLength > MAX_ICON_BYTES) return false;
	const match = /^data:image\/(png|jpeg|webp|svg\+xml);base64,([a-z\d+/]*={0,2})$/i.exec(uri);
	if (!match) return false;
	const payload = match[2];
	if (payload.length % 4 !== 0) return false;
	return true;
}

export function parseManifest(json: unknown): MiniAppManifest | null {
	let value = json;
	if (typeof value === 'string') {
		try {
			value = JSON.parse(value);
		} catch {
			return null;
		}
	}
	if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
	const raw = value as Record<string, unknown>;
	if (raw.v !== 1) return null;
	if (raw.name !== undefined && typeof raw.name !== 'string') return null;
	if (raw.description !== undefined && typeof raw.description !== 'string') return null;
	if (raw.icon !== undefined && (typeof raw.icon !== 'string' || !validDataImage(raw.icon))) {
		return null;
	}
	if (raw.embed !== undefined && raw.embed !== 'card' && raw.embed !== 'link') return null;
	if (raw.height !== undefined && (typeof raw.height !== 'number' || !Number.isFinite(raw.height))) {
		return null;
	}

	return {
		v: 1,
		...(raw.name === undefined ? {} : { name: truncate(raw.name as string, 40) }),
		...(raw.description === undefined
			? {}
			: { description: truncate(raw.description as string, 140) }),
		...(raw.icon === undefined ? {} : { icon: raw.icon as string }),
		embed: (raw.embed as 'card' | 'link' | undefined) ?? 'link',
		...(raw.height === undefined ? {} : { height: clampHeight(raw.height as number) })
	};
}

function isIpLiteral(hostname: string): boolean {
	if (hostname.startsWith('[') && hostname.endsWith(']')) return true;
	const parts = hostname.split('.');
	return (
		parts.length === 4 &&
		parts.every((part) => /^\d{1,3}$/.test(part) && Number(part) >= 0 && Number(part) <= 255)
	);
}

export function isKeyBearingOrigin(origin: string, pageOrigin: string): boolean {
	try {
		const entry = new URL(origin);
		const page = new URL(pageOrigin);
		const hostname = entry.hostname.toLowerCase();
		return (
			entry.origin === page.origin ||
			isIpLiteral(hostname) ||
			RESERVED_KEY_HOSTS.has(hostname)
		);
	} catch {
		return true;
	}
}

export function canEmbed(entry: string, pageOrigin: string, embedMode: unknown): boolean {
	if (embedMode !== 'card' || isKeyBearingOrigin(entry, pageOrigin)) return false;
	try {
		const url = new URL(entry);
		return (
			(url.protocol === 'http:' || url.protocol === 'https:') &&
			url.hostname.toLowerCase().endsWith('.mesh')
		);
	} catch {
		return false;
	}
}

export function safeOpenUrl(url: unknown): string | undefined {
	if (typeof url !== 'string') return undefined;
	try {
		const parsed = new URL(url);
		return parsed.protocol === 'http:' || parsed.protocol === 'https:' ? parsed.href : undefined;
	} catch {
		return undefined;
	}
}

export function acceptBridgeMessage(
	event: BridgeMessageEvent,
	frameWindow: unknown,
	expectedOrigin: string
): BridgeEnvelope | null {
	if (event.source !== frameWindow || event.origin !== expectedOrigin) return null;
	try {
		const serialized = JSON.stringify(event.data);
		if (serialized === undefined || new TextEncoder().encode(serialized).byteLength > MAX_BRIDGE_BYTES) {
			return null;
		}
		const data = JSON.parse(serialized) as Record<string, unknown>;
		if (!data || typeof data !== 'object' || data.mesh !== BRIDGE_VERSION) return null;
		switch (data.type) {
			case 'ready':
				return { mesh: BRIDGE_VERSION, type: 'ready' };
			case 'init':
				return data.v === 1 ? { mesh: BRIDGE_VERSION, type: 'init', v: 1 } : null;
			case 'resize':
				return typeof data.height === 'number' && Number.isFinite(data.height)
					? { mesh: BRIDGE_VERSION, type: 'resize', height: clampHeight(data.height) }
					: null;
			case 'open': {
				const url = safeOpenUrl(data.url);
				return url ? { mesh: BRIDGE_VERSION, type: 'open', url } : null;
			}
			default:
				return null;
		}
	} catch {
		return null;
	}
}

export function sendBridgeMessage(
	frameWindow: { postMessage(message: HostToAppMessage, targetOrigin: string): void },
	message: HostToAppMessage,
	expectedOrigin: string
): void {
	frameWindow.postMessage(message, expectedOrigin);
}

function sameService(a: Record<string, unknown>, b: DirectoryService): boolean {
	return a.name === b.name && a.protocol === b.protocol && a.ip === b.ip && a.port === b.port;
}

export function parseAppsResponse(json: unknown, directoryServices: DirectoryService[]): MiniAppRecord[] {
	if (!json || typeof json !== 'object' || Array.isArray(json)) return [];
	const response = json as Record<string, unknown>;
	if (response.version !== 1 || !Array.isArray(response.apps)) return [];

	const apps: MiniAppRecord[] = [];
	for (const candidate of response.apps) {
		if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) continue;
		const raw = candidate as Record<string, unknown>;
		const service = directoryServices.find((svc) => sameService(raw, svc));
		if (!service || !isMiniApp(service)) continue;
		apps.push({
			service,
			manifest: raw.manifest === null ? null : parseManifest(raw.manifest),
			fetchedAtUnixMs:
				typeof raw.fetched_at_unix_ms === 'number' && Number.isFinite(raw.fetched_at_unix_ms)
					? raw.fetched_at_unix_ms
					: 0,
			stale: raw.stale === true,
			error: typeof raw.error === 'string' ? raw.error : null
		});
	}
	return apps;
}
