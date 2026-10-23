<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	type OwnerKey = { publicHex: string; seedHex: string };
	type ClaimHttp = { status: number; body: string };

	let gateway = $state('');
	let cardNodeId = $state('');
	let cardSecret = $state('');
	let owner = $state<OwnerKey | null>(null);
	let challenge = $state<string | null>(null);
	let outcome = $state<string | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);

	const claimed = $derived(outcome !== null && !outcome.includes('"error"'));

	function explain(raw: string): string {
		if (raw.includes('owner_claim_not_hosted')) {
			return 'This box is not hosting the claim service yet. The press on the case has nothing to arm. SSH is still the installer key, not this page.';
		}
		if (raw.includes('unreachable') || raw.includes('cannot reach')) return raw;
		try {
			const parsed = JSON.parse(raw) as { error?: string };
			if (parsed.error) return parsed.error;
		} catch {
			/* body is already prose */
		}
		return raw;
	}

	async function makeKey() {
		error = null;
		outcome = null;
		challenge = null;
		owner = await invoke<OwnerKey>('claim_owner_key');
	}

	async function ask() {
		if (!owner) return;
		busy = true;
		error = null;
		outcome = null;
		challenge = null;
		try {
			const resp = await invoke<ClaimHttp>('claim_request', {
				input: { gateway: gateway.trim(), ownerPublicHex: owner.publicHex }
			});
			if (resp.status !== 200) {
				error = explain(resp.body);
				return;
			}
			challenge = resp.body;
		} catch (e) {
			error = explain(e instanceof Error ? e.message : String(e));
		} finally {
			busy = false;
		}
	}

	async function prove() {
		if (!owner || !challenge) return;
		busy = true;
		error = null;
		try {
			const resp = await invoke<ClaimHttp>('claim_prove', {
				input: {
					gateway: gateway.trim(),
					ownerSeedHex: owner.seedHex,
					nodeIdHex: cardNodeId.trim(),
					cardSecret,
					challengeJson: challenge
				}
			});
			cardSecret = '';
			if (resp.status !== 200) {
				error = explain(resp.body);
				return;
			}
			outcome = resp.body;
			owner = null;
		} catch (e) {
			cardSecret = '';
			error = explain(e instanceof Error ? e.message : String(e));
		} finally {
			busy = false;
		}
	}
</script>

<div class="shell">
	<header class="top">
		<a class="back" href="/">Lightning Admin</a>
		<h1>Become the admin</h1>
	</header>

	<ol class="steps">
		<li>
			<h2>Stand where you can reach the box</h2>
			<p>
				Use the node's LAN address, the <code>10.42.*.1</code> gateway from the
				discovery list. Joining its open Wi-Fi is enough. This page does not
				put a key in <code>authorized_keys</code>.
			</p>
			<label>
				LAN address
				<input bind:value={gateway} placeholder="10.42.1.1" autocomplete="off" />
			</label>
		</li>
		<li>
			<h2>Make the owner key on this computer</h2>
			<p>The seed stays in this window. It is not sent, and it is not written down here.</p>
			<button type="button" class="scan" onclick={makeKey} disabled={busy}>New owner key</button>
			{#if owner}
				<p class="mono">Owner {owner.publicHex}</p>
			{/if}
		</li>
		<li>
			<h2>Read the card, then press the button on the box</h2>
			<p>
				The card secret is checked on this computer. The box only receives a
				proof. After you ask for a challenge, walk to the router and press the
				setup button within two minutes. That press means you are standing
				there. It does not open WAN SSH.
			</p>
			<label>
				Node id printed on the card
				<input bind:value={cardNodeId} placeholder="64 hex characters" autocomplete="off" />
			</label>
			<label>
				Card secret
				<input type="password" bind:value={cardSecret} autocomplete="off" />
			</label>
			<button type="button" class="scan" disabled={busy || !owner || !gateway.trim()} onclick={ask}>
				Ask the box
			</button>
			{#if challenge}
				<p>The box answered. Press its setup button, then come back.</p>
				<button type="button" class="scan" disabled={busy || !cardSecret || !cardNodeId.trim()} onclick={prove}>
					The button has been pressed
				</button>
			{/if}
		</li>
		<li class:waiting={!claimed}>
			<h2>Issue a biscuit for the next admin</h2>
			<p>
				After this computer is the owner, a later person gets a holder-bound
				biscuit: a scoped grant with a clock, not a root shell. Membership in
				a guild is not that grant. SSH stays the key that was installed with
				the box.
			</p>
			{#if claimed}
				<p>You are the owner of this box. Grant issuance from this page is the next slice; the node has accepted the claim.</p>
			{:else}
				<p class="wait">The grant stays closed until the claim above succeeds.</p>
			{/if}
		</li>
	</ol>

	{#if error}
		<p class="bad" role="alert">{error}</p>
	{/if}
</div>

<style>
	.shell {
		max-width: 44rem;
		margin: 0 auto;
		padding: 1.5rem 1.25rem 3rem;
		color: var(--text);
		font-family: var(--font);
	}
	.top {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 1.5rem;
	}
	.back {
		color: var(--muted);
		text-decoration: none;
		font-size: 0.85rem;
	}
	h1 {
		font-size: 1.4rem;
		font-weight: 650;
		letter-spacing: -0.02em;
		margin: 0;
	}
	.steps {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	li {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 1rem 1rem 1.1rem;
	}
	li.waiting {
		opacity: 0.72;
	}
	h2 {
		font-size: 1rem;
		margin: 0 0 0.4rem;
	}
	p {
		margin: 0 0 0.7rem;
		color: var(--muted);
		line-height: 1.45;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.8rem;
		color: var(--muted);
		margin-bottom: 0.6rem;
	}
	input {
		font: inherit;
		font-family: var(--mono);
		font-size: 0.85rem;
		color: var(--text);
		background: var(--bg);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		padding: 0.45rem 0.55rem;
	}
	.scan {
		font: inherit;
		background: var(--primary-bg);
		color: var(--primary-fg);
		border: 1px solid var(--primary-border);
		border-radius: var(--radius);
		padding: 0.4rem 0.75rem;
		cursor: pointer;
	}
	.scan:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.mono {
		font-family: var(--mono);
		font-size: 0.75rem;
		word-break: break-all;
		color: var(--text);
	}
	.wait {
		margin: 0;
	}
	.bad {
		margin-top: 1rem;
		color: var(--bad);
	}
	code {
		font-family: var(--mono);
		font-size: 0.85em;
	}
</style>
