<!--
	Full rung-1 identity lifecycle, tucked inside the Hero's "Your identity"
	expander (bead c3f — the CustodyNotice lives here now, verbatim, still a hard
	requirement). Create-with-name, rename, reveal the full key, export/import a
	recovery phrase. All state is derived from the shared identityStore, which
	derives status from the live directory — so a key that already exists is never
	met with "create one again". No external hosts; clipboard + crypto both work
	in the insecure plain-HTTP context.
-->
<script lang="ts">
	import { identityStore } from '$lib/identity/store.svelte';
	import { parseSecretInput } from '$lib/identity/mnemonic';
	import { copyText } from '$lib/clipboard';
	import { isForeignOrigin, canonicalUrl } from '$lib/origin';
	import CustodyNotice from './CustodyNotice.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';

	const inputClass =
		'h-8 w-full rounded-md border border-input bg-background px-2.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring';

	// Create / rename share a name field. Seed it from the current label.
	let nameInput = $state(identityStore.label);
	let learnWhyOpen = $state(false);
	let revealOpen = $state(false);
	let exportOpen = $state(false);
	let importOpen = $state(false);
	let importText = $state('');
	let importError = $state('');
	let busy = $state(false);

	let copied = $state<'' | 'key' | 'phrase' | 'hex'>('');

	const hasKey = $derived(!!identityStore.identity);
	const fullKey = $derived(identityStore.publicKeyHex);

	// Origin guard — only meaningful in the browser.
	const foreignOrigin = $derived.by(() => {
		if (typeof window === 'undefined') return false;
		return isForeignOrigin(window.location.hostname);
	});
	const canonical = $derived.by(() =>
		typeof window === 'undefined' ? '' : canonicalUrl(window.location.protocol)
	);

	// Recovery secret is only read on demand (never eagerly in state).
	let secret = $state<{ mnemonic: string; hex: string } | undefined>(undefined);
	function revealSecret() {
		exportOpen = !exportOpen;
		if (exportOpen && !secret) {
			try {
				secret = identityStore.exportSecret();
			} catch {
				secret = undefined;
			}
		}
	}
	const mnemonicWords = $derived(secret ? secret.mnemonic.split(' ') : []);

	async function flashCopy(kind: 'key' | 'phrase' | 'hex', text: string) {
		const ok = await copyText(text);
		if (ok) {
			copied = kind;
			setTimeout(() => (copied = copied === kind ? '' : copied), 1500);
		}
	}

	async function join() {
		busy = true;
		try {
			await identityStore.create(nameInput);
		} finally {
			busy = false;
		}
	}

	async function rename() {
		busy = true;
		try {
			await identityStore.rename(nameInput);
		} finally {
			busy = false;
		}
	}

	async function doImport() {
		importError = '';
		let seed: Uint8Array;
		try {
			seed = parseSecretInput(importText);
		} catch (err) {
			importError = err instanceof Error ? err.message : String(err);
			return;
		}
		if (hasKey) {
			const ok = window.confirm(
				'This replaces the identity currently held in this browser. The old key will be gone unless you saved its recovery phrase. Continue?'
			);
			if (!ok) return;
		}
		busy = true;
		try {
			await identityStore.importSeed(seed);
			importText = '';
			importOpen = false;
			secret = undefined; // force re-read on next export
		} catch (err) {
			importError = err instanceof Error ? err.message : String(err);
		} finally {
			busy = false;
		}
	}
</script>

<div class="flex flex-col gap-4">
	{#if foreignOrigin}
		<p class="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning">
			You're here by IP address, so this page can't see an identity you made at
			<!-- Canonical hello.mesh origin (different host), not an app route. -->
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={canonical} class="font-medium underline underline-offset-2">hello.mesh</a>. Open it
			there to be recognized.
		</p>
	{/if}

	{#if !hasKey}
		<!-- Anonymous: create with a name. -->
		<div class="flex flex-col gap-2">
			<label class="text-sm font-medium" for="identity-name">Introduce yourself</label>
			<div class="flex gap-2">
				<input
					id="identity-name"
					class={inputClass}
					placeholder="Your name"
					maxlength="48"
					bind:value={nameInput}
					onkeydown={(e) => e.key === 'Enter' && nameInput.trim() && join()}
				/>
				<Button size="sm" disabled={busy || !nameInput.trim()} onclick={join}>Join</Button>
			</div>
			<p class="text-xs text-muted-foreground">
				Your name and a key are created and held in this browser.
				<button
					type="button"
					class="underline underline-offset-2 hover:text-foreground"
					onclick={() => (learnWhyOpen = !learnWhyOpen)}>Learn why</button
				>
			</p>
			{#if learnWhyOpen}
				<CustodyNotice />
			{/if}
		</div>

		<Collapsible.Root bind:open={importOpen}>
			<Collapsible.Trigger
				class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
			>
				Already have a recovery phrase? Restore it
			</Collapsible.Trigger>
			<Collapsible.Content class="mt-2 flex flex-col gap-2">
				<textarea
					class="{inputClass} h-auto min-h-16 py-1.5 font-mono text-xs"
					placeholder="Paste your 24-word recovery phrase or 64-character hex seed"
					bind:value={importText}></textarea>
				{#if importError}
					<p class="text-xs text-destructive">{importError}</p>
				{/if}
				<Button
					size="sm"
					variant="outline"
					class="w-fit"
					disabled={busy || !importText.trim()}
					onclick={doImport}
				>
					Restore identity
				</Button>
			</Collapsible.Content>
		</Collapsible.Root>
	{:else}
		<!-- Has a key: rename, reveal, export, import. -->
		<CustodyNotice />

		<div class="flex flex-col gap-2">
			<label class="text-sm font-medium" for="identity-rename">Your name</label>
			<div class="flex gap-2">
				<input
					id="identity-rename"
					class={inputClass}
					placeholder="Your name"
					maxlength="48"
					bind:value={nameInput}
					onkeydown={(e) => e.key === 'Enter' && rename()}
				/>
				<Button
					size="sm"
					variant="outline"
					disabled={busy || nameInput.trim() === identityStore.label}
					onclick={rename}>Rename</Button
				>
			</div>
		</div>

		<!-- Reveal the full public key. -->
		<div class="flex flex-col gap-2">
			<Button
				size="sm"
				variant="ghost"
				class="w-fit px-0 text-muted-foreground hover:bg-transparent hover:text-foreground"
				onclick={() => (revealOpen = !revealOpen)}
			>
				{revealOpen ? 'Hide' : 'Show'} full key
			</Button>
			{#if revealOpen}
				<div class="flex flex-col gap-1.5">
					<code class="block rounded bg-muted px-2 py-1.5 font-mono text-xs break-all select-all"
						>{fullKey}</code
					>
					<Button
						size="xs"
						variant="outline"
						class="w-fit"
						onclick={() => flashCopy('key', fullKey)}
					>
						{copied === 'key' ? 'Copied' : 'Copy key'}
					</Button>
				</div>
			{/if}
		</div>

		<!-- Export: recovery phrase + raw hex, gated behind a click. -->
		<div class="flex flex-col gap-2">
			<Button
				size="sm"
				variant="ghost"
				class="w-fit px-0 text-muted-foreground hover:bg-transparent hover:text-foreground"
				onclick={revealSecret}
			>
				{exportOpen ? 'Hide' : 'Show'} recovery phrase
			</Button>
			{#if exportOpen && secret}
				<div class="flex flex-col gap-2 rounded-md border border-warning/30 bg-warning/10 p-3">
					<p class="text-xs text-warning">
						<strong class="font-semibold">Anyone with these words is you.</strong> Write them down offline.
						Never paste them into anything you don't trust.
					</p>
					<ol class="grid grid-cols-2 gap-x-4 gap-y-0.5 font-mono text-xs sm:grid-cols-3">
						{#each mnemonicWords as word, i (i)}
							<li class="flex gap-1.5">
								<span class="w-5 text-right text-muted-foreground tabular-nums">{i + 1}.</span>
								<span>{word}</span>
							</li>
						{/each}
					</ol>
					<div class="flex flex-wrap gap-2">
						<Button
							size="xs"
							variant="outline"
							onclick={() => flashCopy('phrase', secret!.mnemonic)}
						>
							{copied === 'phrase' ? 'Copied' : 'Copy phrase'}
						</Button>
						<Button size="xs" variant="ghost" onclick={() => flashCopy('hex', secret!.hex)}>
							{copied === 'hex' ? 'Copied' : 'Copy raw hex'}
						</Button>
					</div>
				</div>
			{/if}
		</div>

		<!-- Import / restore a different key. -->
		<Collapsible.Root bind:open={importOpen}>
			<Collapsible.Trigger
				class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
			>
				Restore a different identity
			</Collapsible.Trigger>
			<Collapsible.Content class="mt-2 flex flex-col gap-2">
				<textarea
					class="{inputClass} h-auto min-h-16 py-1.5 font-mono text-xs"
					placeholder="Paste a 24-word recovery phrase or 64-character hex seed"
					bind:value={importText}></textarea>
				{#if importError}
					<p class="text-xs text-destructive">{importError}</p>
				{/if}
				<Button
					size="sm"
					variant="outline"
					class="w-fit"
					disabled={busy || !importText.trim()}
					onclick={doImport}
				>
					Restore identity
				</Button>
			</Collapsible.Content>
		</Collapsible.Root>
	{/if}
</div>
