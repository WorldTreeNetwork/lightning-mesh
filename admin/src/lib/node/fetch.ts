import { invoke } from '@tauri-apps/api/core';
import type { LanPath, LanTarget } from './lan';

const pending = new Map<string, Promise<unknown>>();

export async function fetchLanJson(address: string, path: LanPath, scopeId = 0): Promise<unknown> {
	const key = `${scopeId}|${address}|${path}`;
	const existing = pending.get(key);
	if (existing) return existing;
	const request = invoke<unknown>('fetch_lan_json', { address, path, scopeId }).then(
		(value) => value,
		(cause: unknown) => {
			pending.delete(key);
			throw cause;
		}
	);
	pending.set(key, request);
	void request.then(() => {
		setTimeout(() => pending.delete(key), 8000);
	});
	return request;
}

export type FetchSubject = {
	id: string;
	name: string;
	heard: boolean;
	target: LanTarget | null;
};

export type FetchRow = FetchSubject & {
	document: unknown | null;
	error: string | null;
};

export async function fetchDocuments(subjects: FetchSubject[], path: LanPath): Promise<FetchRow[]> {
	return Promise.all(
		subjects.map(async (subject) => {
			if (!subject.target) {
				return {
					...subject,
					document: null,
					error: subject.heard ? 'No LAN address on the last scan.' : null
				};
			}
			try {
				const document = await fetchLanJson(subject.target.address, path, subject.target.scopeId);
				return { ...subject, document, error: null };
			} catch (cause) {
				const error = cause instanceof Error ? cause.message : String(cause);
				return { ...subject, document: null, error };
			}
		})
	);
}
