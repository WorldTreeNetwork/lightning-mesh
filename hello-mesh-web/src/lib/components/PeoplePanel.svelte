<!--
	People: everyone who has introduced themselves on this network. No accounts —
	just a name and a key each person holds themselves. Rendered as a chip wall,
	most-recently-seen first, your own chip pinned first with a "you" badge. A
	recency dot (never an "online" light) shows how long ago each person last
	announced; dormant identities fold into a disclosure so the directory stops
	looking like it never forgets. Clicking a chip opens a popover with the full
	name, full key (copyable), and recency — the raw 64-hex is NEVER a handle.
-->
<script lang="ts">
	import { directoryStore } from '$lib/directory/store.svelte';
	import { identityStore } from '$lib/identity/store.svelte';
	import { groupPeople, type Person } from '$lib/directory/people';
	import { presenceColorVar } from '$lib/directory/presence';
	import { copyText } from '$lib/clipboard';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { Users, ChevronRight } from '@lucide/svelte';

	// Re-derive on directory changes AND on identity changes (so your own chip
	// appears the moment you announce). A slow ticker keeps recency labels fresh.
	let now = $state(Date.now());
	$effect(() => {
		const t = setInterval(() => (now = Date.now()), 30_000);
		return () => clearInterval(t);
	});

	const identities = $derived(directoryStore.directory?.identities ?? []);
	const selfKey = $derived(identityStore.publicKeyHex || undefined);
	const groups = $derived(groupPeople(identities, selfKey, now));

	let dormantOpen = $state(false);
	let copiedKey = $state('');

	async function copyKey(key: string) {
		if (await copyText(key)) {
			copiedKey = key;
			setTimeout(() => (copiedKey = copiedKey === key ? '' : copiedKey), 1500);
		}
	}
</script>

{#snippet chip(p: Person)}
	<Popover.Root>
		<Popover.Trigger
			class="flex items-center gap-2 rounded-full border px-3 py-1.5 text-sm transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none {p.isSelf
				? 'border-primary/40 ring-1 ring-primary/30'
				: 'border-border bg-background'}"
		>
			{#if p.tier !== 'unknown'}
				<span
					class="inline-flex size-2 shrink-0 rounded-full {p.tier === 'active'
						? 'presence-dot-active'
						: ''}"
					style="background: {presenceColorVar(p.tier)}"
					aria-hidden="true"
				></span>
			{/if}
			<span class="font-medium">{p.displayName}</span>
			<span class="font-mono text-xs text-muted-foreground">{p.shortKey}…</span>
			{#if p.isSelf}
				<span
					class="rounded-full bg-primary px-1.5 py-0.5 text-[10px] font-semibold text-primary-foreground"
					>you</span
				>
			{/if}
		</Popover.Trigger>
		<Popover.Content class="w-80">
			<div class="flex flex-col gap-3">
				<div class="flex items-center gap-2">
					{#if p.tier !== 'unknown'}
						<span
							class="inline-flex size-2.5 shrink-0 rounded-full {p.tier === 'active'
								? 'presence-dot-active'
								: ''}"
							style="background: {presenceColorVar(p.tier)}"
							aria-hidden="true"
						></span>
					{/if}
					<span class="font-semibold">{p.displayName}</span>
					{#if p.isSelf}
						<span class="text-xs text-muted-foreground">(you)</span>
					{/if}
				</div>
				<p class="text-xs text-muted-foreground">
					{p.recency || 'Last seen unknown'}
				</p>
				<div class="flex flex-col gap-1.5">
					<span class="text-xs font-medium text-muted-foreground">Public key</span>
					<code class="block rounded bg-muted px-2 py-1.5 font-mono text-xs break-all select-all"
						>{p.key}</code
					>
					<button
						type="button"
						class="w-fit rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
						onclick={() => copyKey(p.key)}
					>
						{copiedKey === p.key ? 'Copied' : 'Copy key'}
					</button>
				</div>
			</div>
		</Popover.Content>
	</Popover.Root>
{/snippet}

<section
	class="flex flex-col gap-4 rounded-lg border border-border bg-card p-4 text-card-foreground"
>
	<header class="flex flex-col gap-1">
		<div class="flex items-center gap-2">
			<Users class="size-4 text-muted-foreground" aria-hidden="true" />
			<h2 class="text-lg font-semibold">People</h2>
		</div>
		<p class="text-sm text-muted-foreground">
			Anyone who's introduced themselves on this network. No accounts — just a name and a key they
			hold themselves.
		</p>
	</header>

	{#if !directoryStore.loaded}
		<p class="text-sm text-muted-foreground">Loading…</p>
	{:else if groups.present.length === 0 && groups.dormant.length === 0}
		<p class="text-sm text-muted-foreground">
			No one has introduced themselves yet. Be the first — set up your identity above.
		</p>
	{:else}
		{#if groups.present.length > 0}
			<div class="flex flex-wrap gap-2">
				{#each groups.present as p (p.key)}
					{@render chip(p)}
				{/each}
			</div>
		{/if}

		{#if groups.dormant.length > 0}
			<Collapsible.Root bind:open={dormantOpen}>
				<Collapsible.Trigger
					class="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
				>
					<ChevronRight
						class="size-4 transition-transform duration-200 {dormantOpen ? 'rotate-90' : ''}"
						aria-hidden="true"
					/>
					Not seen recently ({groups.dormant.length})
				</Collapsible.Trigger>
				<Collapsible.Content class="mt-2">
					<div class="flex flex-wrap gap-2">
						{#each groups.dormant as p (p.key)}
							{@render chip(p)}
						{/each}
					</div>
				</Collapsible.Content>
			</Collapsible.Root>
		{/if}
	{/if}
</section>
