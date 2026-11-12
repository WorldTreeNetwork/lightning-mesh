import { asRadioDocument, type RadioDocument } from '$lib/node/documents';

export type LightningDetail = {
	link: string;
	strength: string;
	version: string;
	hops: string;
	status: string;
	statusTitle: string;
};

function record(value: unknown): Record<string, unknown> | null {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
	return value as Record<string, unknown>;
}

/** A software version string. A bare schema integer is not a mesh version. */
export function publishedVersion(node: unknown): string | null {
	const body = record(node);
	if (!body) return null;
	for (const key of ['mesh_version', 'version']) {
		const value = body[key];
		if (typeof value !== 'string') continue;
		const text = value.trim();
		if (!text || /^\d+$/.test(text)) continue;
		return text;
	}
	return null;
}

export function healthStatus(body: unknown): string | null {
	const value = record(body);
	const status = value?.status;
	return typeof status === 'string' && status.trim() ? status.trim() : null;
}

export function strongestSignal(radio: RadioDocument | null): number | null {
	let best: number | null = null;
	for (const station of radio?.stations ?? []) {
		if (station.signal_dbm === null) continue;
		if (best === null || station.signal_dbm > best) best = station.signal_dbm;
	}
	return best;
}

export function lightningDetail(input: {
	iface: string;
	state: string;
	node: unknown | null;
	radio: unknown | null;
	health: unknown | null;
	healthError: string | null;
}): LightningDetail {
	let radio: RadioDocument | null = null;
	if (input.radio) {
		try {
			radio = asRadioDocument(input.radio);
		} catch {
			radio = null;
		}
	}
	const channel = radio?.channel === null || radio?.channel === undefined ? '' : ` · ch ${radio.channel}`;
	const link = radio?.mesh_if ? `${radio.mesh_if}${channel}` : input.iface || '—';
	const signal = strongestSignal(radio);
	const strength = signal === null ? '—' : `${signal} dBm`;
	const version = publishedVersion(input.node) ?? '—';
	const health = healthStatus(input.health);
	const status = health ? `${input.state} · ${health}` : input.healthError ? `${input.state} · no answer` : input.state;
	return {
		link,
		strength,
		version,
		hops: '—',
		status,
		statusTitle: input.healthError ?? ''
	};
}
