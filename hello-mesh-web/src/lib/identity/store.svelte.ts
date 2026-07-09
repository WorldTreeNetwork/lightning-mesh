// Shared rung-1 identity state. This is the single owner of "who am I on the
// mesh": it loads the browser-held key, announces it (challenge -> sign ->
// POST, ALWAYS with the stored label), and derives the user-facing status from
// the live directory poll rather than a one-shot flag — so a key that already
// exists is recognized on every load instead of prompting "create one" again.
//
// A visibility-aware heartbeat keeps the identity fresh: while the page is
// visible we re-announce every few minutes (and sooner if our directory entry
// is missing or its last-seen has gone stale) so the mesh keeps showing the
// person as "here now". A label-less announce could clobber the name on older
// daemons, so every announce carries the label.

import { browser } from '$app/environment';
import { fetchChallenge, submitIdentity } from './api';
import { generateKeyPair, keyPairFromSecret, publicKeyHex, signChallengeHex } from './keys';
import { loadIdentity, saveIdentity, type StoredIdentity } from './storage';
import { seedToHex, seedToMnemonic } from './mnemonic';
import { deriveStatus, statusMessage, type IdentityStatus } from './status';
import { directoryStore } from '$lib/directory/store.svelte';
import { ACTIVE_WINDOW_MS } from '$lib/directory/presence';

const HEARTBEAT_MS = 3 * 60_000; // routine re-announce cadence while visible
const TICK_MS = 30_000; // how often the heartbeat re-evaluates
const RETRY_MIN_GAP_MS = 15_000; // don't hammer the node while a spool settles

export interface ExportedSecret {
	mnemonic: string;
	hex: string;
}

class IdentityStore {
	identity = $state<StoredIdentity | undefined>(undefined);
	loaded = $state(false);
	/** True only while a POST /api/identity is in flight. */
	announcing = $state(false);
	lastAnnounceErrored = $state(false);
	announceError = $state('');
	/** ms epoch when we began trying to place the CURRENT key in the directory. */
	private announceStartedAt = $state<number | undefined>(undefined);
	private lastAnnounceAt = $state<number | undefined>(undefined);

	get publicKeyHex(): string {
		return this.identity ? publicKeyHex(this.identity.publicKey) : '';
	}

	get label(): string {
		return this.identity?.label ?? '';
	}

	/** Our identity row in the directory the node just served, if present. */
	get directoryEntry() {
		const key = this.publicKeyHex.toLowerCase();
		if (!key) return undefined;
		return directoryStore.directory?.identities.find((i) => i.username.toLowerCase() === key);
	}

	get inDirectory(): boolean {
		return this.directoryEntry !== undefined;
	}

	get nameInSync(): boolean {
		const entry = this.directoryEntry;
		if (!entry) return false;
		// A blank local label means we never named ourselves; the daemon's
		// short-key fallback display_name is then "in sync" by definition.
		if (!this.label) return true;
		return entry.display_name === this.label;
	}

	get status(): IdentityStatus {
		return deriveStatus({
			haveLocalKey: !!this.identity,
			inDirectory: this.inDirectory,
			nameInSync: this.nameInSync,
			lastAnnounceErrored: this.lastAnnounceErrored,
			announcingForMs:
				this.announceStartedAt === undefined ? undefined : Date.now() - this.announceStartedAt
		});
	}

	/** The name to show for "you" — settled directory name, else local label. */
	get displayName(): string {
		return this.directoryEntry?.display_name || this.label;
	}

	get statusMessage(): string {
		return statusMessage(this.status, this.displayName);
	}

	async init(): Promise<void> {
		if (!browser) return;
		this.identity = await loadIdentity();
		this.loaded = true;
		if (this.identity) {
			this.announceStartedAt = Date.now();
			await this.announce();
		}
	}

	/** challenge -> sign -> POST, always including the stored label. */
	async announce(): Promise<void> {
		if (!this.identity || this.announcing) return;
		this.announcing = true;
		this.lastAnnounceErrored = false;
		this.announceError = '';
		try {
			const challenge = await fetchChallenge();
			const sig = await signChallengeHex(this.identity.secretKey, challenge);
			await submitIdentity({
				pubkey: publicKeyHex(this.identity.publicKey),
				sig,
				challenge,
				label: this.identity.label || undefined
			});
			this.lastAnnounceAt = Date.now();
		} catch (err) {
			this.lastAnnounceErrored = true;
			this.announceError = err instanceof Error ? err.message : String(err);
		} finally {
			this.announcing = false;
		}
	}

	async create(name: string): Promise<void> {
		const keyPair = await generateKeyPair();
		const stored: StoredIdentity = {
			publicKey: keyPair.publicKey,
			secretKey: keyPair.secretKey,
			createdAt: Date.now(),
			label: name.trim() || undefined
		};
		await saveIdentity(stored);
		this.identity = stored;
		this.announceStartedAt = Date.now();
		await this.announce();
	}

	async rename(name: string): Promise<void> {
		if (!this.identity) return;
		const stored: StoredIdentity = { ...this.identity, label: name.trim() || undefined };
		await saveIdentity(stored);
		this.identity = stored;
		await this.announce();
	}

	/** Replace the current key from an imported 32-byte seed (+ optional name). */
	async importSeed(seed: Uint8Array, name?: string): Promise<void> {
		const keyPair = keyPairFromSecret(seed);
		const stored: StoredIdentity = {
			publicKey: keyPair.publicKey,
			secretKey: keyPair.secretKey,
			createdAt: Date.now(),
			label: (name ?? this.identity?.label ?? '').trim() || undefined
		};
		await saveIdentity(stored);
		this.identity = stored;
		this.announceStartedAt = Date.now();
		await this.announce();
	}

	/** Recovery phrase + raw hex for the export UI. Throws if anonymous. */
	exportSecret(): ExportedSecret {
		if (!this.identity) throw new Error('no identity to export');
		return {
			mnemonic: seedToMnemonic(this.identity.secretKey),
			hex: seedToHex(this.identity.secretKey)
		};
	}

	/** Manual "re-announce" for the error path. */
	async reannounce(): Promise<void> {
		if (this.announceStartedAt === undefined) this.announceStartedAt = Date.now();
		await this.announce();
	}

	/** Heartbeat decision for one tick; returns true if it should re-announce now. */
	private shouldHeartbeat(now: number): boolean {
		if (!this.identity || this.announcing) return false;
		const sinceLast = this.lastAnnounceAt === undefined ? Infinity : now - this.lastAnnounceAt;
		if (sinceLast < RETRY_MIN_GAP_MS) return false;

		// Not yet visible in the directory — keep trying to land the announce.
		if (!this.inDirectory) return true;

		// Our directory entry exists but its last-seen has aged out of "here now".
		const lastSeen = this.directoryEntry?.last_seen_unix;
		if (lastSeen !== undefined && now - lastSeen > ACTIVE_WINDOW_MS) return true;

		// Otherwise the routine cadence.
		return sinceLast >= HEARTBEAT_MS;
	}

	/**
	 * Start the visibility-aware heartbeat. Call from a component `$effect`;
	 * returns a teardown. Pauses while the tab is hidden, fires immediately on
	 * becoming visible again.
	 */
	startHeartbeat(): () => void {
		if (!browser) return () => {};
		const tick = () => {
			if (document.visibilityState === 'hidden') return;
			if (this.shouldHeartbeat(Date.now())) void this.announce();
		};
		const onVisible = () => {
			if (document.visibilityState === 'visible') tick();
		};
		const interval = setInterval(tick, TICK_MS);
		document.addEventListener('visibilitychange', onVisible);
		return () => {
			clearInterval(interval);
			document.removeEventListener('visibilitychange', onVisible);
		};
	}
}

export const identityStore = new IdentityStore();

/** Convenience for a page `$effect`: kicks off load + heartbeat, returns teardown. */
export function startIdentity(): () => void {
	if (!browser) return () => {};
	void identityStore.init();
	return identityStore.startHeartbeat();
}
