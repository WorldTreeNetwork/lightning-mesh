// Shared directory poller. One /api/directory poll feeds every panel that
// needs mesh state (the directory grid and the mDNS services panel) so we
// don't run duplicate intervals. Runes-based, browser-only; call
// `startDirectoryPolling()` from a component `$effect` and read the fields.

import { browser } from '$app/environment';
import { fetchDirectory, type Directory } from './api';
import { mockDirectory } from './fixtures';

const POLL_INTERVAL_MS = 5000;

/** Dev/demo: `?mockDirectory=1` renders the whole page from a fixture. */
function mockDirectoryEnabled(): boolean {
	return browser && new URLSearchParams(window.location.search).get('mockDirectory') === '1';
}

class DirectoryStore {
	directory = $state<Directory | undefined>(undefined);
	reconnecting = $state(false);
	loaded = $state(false);

	async poll() {
		try {
			this.directory = await fetchDirectory();
			this.reconnecting = false;
		} catch {
			// Keep last-good directory; surface a subtle hint, never a hard error.
			this.reconnecting = true;
		} finally {
			this.loaded = true;
		}
	}
}

export const directoryStore = new DirectoryStore();

/** Begin polling; returns a teardown to hand back from `$effect`. */
export function startDirectoryPolling(): () => void {
	if (!browser) return () => {};
	if (mockDirectoryEnabled()) {
		// Refresh the fixture on a slow tick so relative recency labels keep moving.
		directoryStore.directory = mockDirectory();
		directoryStore.loaded = true;
		const interval = setInterval(() => {
			directoryStore.directory = mockDirectory();
		}, 15_000);
		return () => clearInterval(interval);
	}
	directoryStore.poll();
	const interval = setInterval(() => directoryStore.poll(), POLL_INTERVAL_MS);
	return () => clearInterval(interval);
}
