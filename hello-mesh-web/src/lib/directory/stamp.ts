// Phone stamp of last-known node coordinates (bead mjolnir-mesh-6hn.5).
// Pure helpers: nearby-narrowed list, GPS gate, domain-separated signing.
// POST lives here so the RoutersPanel control stays a thin UI.

import type { Directory, DirectoryNeighbor, DirectoryNode } from './api';
import type { TopoGraph } from '$lib/topology/graph';
import { fetchChallenge } from '$lib/identity/api';
import { publicKeyHex, signMessage } from '$lib/identity/keys';

/** Must match `COORDINATE_STAMP_DOMAIN` in crates/mjolnir-hello/src/routes.rs. */
export const COORDINATE_STAMP_DOMAIN = 'mjolnir-coordinate-stamp:v1';

export const MICRODEGREES = 10_000_000;

export interface StampTarget {
	nodeId: string;
	name: string;
	fallback: string;
	subnet: string | null;
	isSelf: boolean;
	lat?: number;
	lon?: number;
	alt?: number;
	stampedAt?: number;
	stamper?: string;
	/** Radio strength of this neighbor as heard by the associated node, when known. */
	signalDbm?: number;
}

export type StampIntent = 'confirm' | 'cancel';

function fallbackLabel(nodeId: string, subnet: string | null): string {
	const octet = subnet?.split('.')?.[2];
	if (octet) return `Router ${octet}`;
	return nodeId.length > 8 ? nodeId.slice(0, 8) : nodeId;
}

function hostOf(addr: string | undefined): string | undefined {
	if (!addr) return undefined;
	const idx = addr.lastIndexOf(':');
	return idx === -1 ? addr : addr.slice(0, idx);
}

function coordsOf(n: DirectoryNode | DirectoryNeighbor): Pick<
	StampTarget,
	'lat' | 'lon' | 'alt' | 'stampedAt' | 'stamper'
> {
	return {
		lat: n.lat,
		lon: n.lon,
		alt: n.alt,
		stampedAt: n.stamped_at,
		stamper: n.stamper
	};
}

/** Algebraically greater dBm is stronger (−40 beats −80). Missing sorts last. */
function signalDbmFor(neighbor: DirectoryNeighbor, graph?: TopoGraph): number | undefined {
	if (!graph) return undefined;
	const self = graph.nodes.find((n) => n.isSelf);
	if (!self?.radio?.stations?.length) return undefined;
	const peer = graph.nodes.find(
		(n) =>
			n.nodeId === neighbor.node_id ||
			(!!n.subnet && n.subnet === neighbor.subnet) ||
			n.key === (hostOf(neighbor.addrs[0]) ?? '')
	);
	const mac = peer?.radio?.mesh_mac?.toLowerCase();
	if (!mac) return undefined;
	return self.radio.stations.find((s) => s.mac.toLowerCase() === mac)?.signal_dbm;
}

/**
 * Associated node first, then neighbors ranked by nearby radio strength when
 * known. Empty / null directory → empty list (UI must not POST).
 */
export function stampTargets(directory: Directory | undefined, graph?: TopoGraph): StampTarget[] {
	if (!directory?.node) return [];
	const self: StampTarget = {
		nodeId: directory.node.node_id,
		name: directory.node.name?.trim() ?? '',
		fallback: fallbackLabel(directory.node.node_id, directory.node.subnet),
		subnet: directory.node.subnet,
		isSelf: true,
		...coordsOf(directory.node)
	};
	const neighbors: StampTarget[] = directory.neighbors.map((n) => ({
		nodeId: n.node_id,
		name: n.name?.trim() ?? '',
		fallback: fallbackLabel(n.node_id, n.subnet),
		subnet: n.subnet,
		isSelf: false,
		...coordsOf(n),
		signalDbm: signalDbmFor(n, graph)
	}));
	neighbors.sort((a, b) => {
		const as = a.signalDbm ?? Number.NEGATIVE_INFINITY;
		const bs = b.signalDbm ?? Number.NEGATIVE_INFINITY;
		return bs - as;
	});
	return [self, ...neighbors];
}

/** Finite WGS84 pair from geolocation or typed fields. Empty / NaN is a miss. */
export function hasGps(lat: unknown, lon: unknown): boolean {
	return (
		typeof lat === 'number' &&
		typeof lon === 'number' &&
		Number.isFinite(lat) &&
		Number.isFinite(lon) &&
		lat >= -90 &&
		lat <= 90 &&
		lon >= -180 &&
		lon <= 180
	);
}

/**
 * Gate for POST /api/coordinate-stamp. Cancel, empty list, no selection, or
 * missing GPS with nothing typed → do not write.
 */
export function shouldPostStamp(args: {
	intent: StampIntent;
	targets: StampTarget[];
	selectedNodeId?: string;
	lat?: number | null;
	lon?: number | null;
}): boolean {
	if (args.intent !== 'confirm') return false;
	if (args.targets.length === 0) return false;
	if (!args.selectedNodeId) return false;
	if (!args.targets.some((t) => t.nodeId === args.selectedNodeId)) return false;
	return hasGps(args.lat, args.lon);
}

/** Half away from zero — matches Rust `f64::round` used by hello. */
export function roundHalfAwayFromZero(n: number): number {
	return Math.sign(n) * Math.round(Math.abs(n));
}

export function degreesToE7(deg: number): number {
	return roundHalfAwayFromZero(deg * MICRODEGREES);
}

export function metresToMm(m: number): number {
	return roundHalfAwayFromZero(m * 1000);
}

export function coordinateStampSigningBytes(args: {
	challenge: string;
	nodeId: string;
	lat: number;
	lon: number;
	alt?: number;
	stampedAt: number;
}): Uint8Array {
	const latE7 = degreesToE7(args.lat);
	const lonE7 = degreesToE7(args.lon);
	const altPart = args.alt === undefined ? '' : String(metresToMm(args.alt));
	return new TextEncoder().encode(
		`${COORDINATE_STAMP_DOMAIN}\n${args.challenge}\n${args.nodeId}\n${latE7}\n${lonE7}\n${altPart}\n${args.stampedAt}`
	);
}

export interface SubmitCoordinateStampParams {
	secretKey: Uint8Array;
	publicKey: Uint8Array;
	nodeId: string;
	lat: number;
	lon: number;
	alt?: number;
	stampedAt?: number;
}

export async function submitCoordinateStamp(
	params: SubmitCoordinateStampParams,
	fetchImpl: typeof fetch = fetch
): Promise<void> {
	const challenge = await fetchChallenge(fetchImpl);
	const stampedAt = params.stampedAt ?? Math.floor(Date.now() / 1000);
	const sig = signMessage(
		params.secretKey,
		coordinateStampSigningBytes({
			challenge,
			nodeId: params.nodeId,
			lat: params.lat,
			lon: params.lon,
			alt: params.alt,
			stampedAt
		})
	);
	const body: Record<string, unknown> = {
		pubkey: publicKeyHex(params.publicKey),
		sig,
		challenge,
		node_id: params.nodeId,
		lat: params.lat,
		lon: params.lon,
		stamped_at: stampedAt
	};
	if (params.alt !== undefined) body.alt = params.alt;

	const res = await fetchImpl('/api/coordinate-stamp', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!res.ok) {
		const detail = await res.text().catch(() => '');
		throw new Error(`POST /api/coordinate-stamp failed: ${res.status} ${res.statusText} ${detail}`);
	}
}
