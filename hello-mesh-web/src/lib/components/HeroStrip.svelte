<!--
	Orientation strip: tells a first-time visitor what this network is, names the
	router they're actually talking to, and carries their identity chip + the
	"Your identity" expander (which holds the full lifecycle UI and the custody
	notice — bead c3f). Router name falls back to a muted "unnamed" subnet/short-id
	label. Identity status is derived live from the directory, never a stored flag.
-->
<script lang="ts">
	import { directoryStore } from '$lib/directory/store.svelte';
	import { identityStore } from '$lib/identity/store.svelte';
	import { presenceTier } from '$lib/directory/presence';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import IdentityManager from './IdentityManager.svelte';
	import { ChevronRight, UserRound } from '@lucide/svelte';

	let expanderOpen = $state(false);

	const node = $derived(directoryStore.directory?.node);

	// Router name, or a muted fallback derived from the subnet / short node id.
	const routerName = $derived(node?.name?.trim() || '');
	const routerFallback = $derived.by(() => {
		if (!node) return 'this router';
		const octet = node.subnet?.split('.')?.[2];
		if (octet) return `Router ${octet}`;
		return node.node_id.length > 8 ? node.node_id.slice(0, 8) : node.node_id;
	});

	const status = $derived(identityStore.status);
	const chipName = $derived(identityStore.displayName);

	// Presence dot for your own chip when you're settled in the directory.
	const selfTier = $derived.by(() => {
		const entry = identityStore.directoryEntry;
		if (!entry?.last_seen_unix) return undefined;
		return presenceTier(Date.now() - entry.last_seen_unix);
	});
</script>

<section
	class="flex flex-col gap-4 rounded-lg border border-border bg-card p-5 text-card-foreground sm:flex-row sm:items-start sm:justify-between"
>
	<div class="flex flex-col gap-1.5">
		<h1 class="text-xl font-semibold sm:text-2xl">
			{#if node}
				You're connected to
				{#if routerName}
					<span class="text-foreground">{routerName}</span>
				{:else}
					<span class="text-muted-foreground italic">{routerFallback}</span>
				{/if}
			{:else}
				Connecting to the mesh…
			{/if}
		</h1>
		<p class="max-w-prose text-sm text-muted-foreground">
			A community mesh running on the routers around you. No internet required.
		</p>
	</div>

	<!-- Identity chip -->
	<div class="shrink-0">
		<button
			type="button"
			class="flex items-center gap-2 rounded-full border border-border bg-background px-3 py-1.5 text-sm transition-colors hover:bg-muted"
			aria-expanded={expanderOpen}
			onclick={() => (expanderOpen = !expanderOpen)}
		>
			{#if status === 'present'}
				<span
					class="relative inline-flex size-2 rounded-full {selfTier === 'active'
						? 'presence-dot-active'
						: ''}"
					style="background: var(--presence-active)"
					aria-hidden="true"
				></span>
				<span class="font-medium">You're {chipName} here</span>
			{:else if status === 'anonymous'}
				<UserRound class="size-3.5 text-muted-foreground" aria-hidden="true" />
				<span class="text-muted-foreground">Set up your identity</span>
			{:else if status === 'error'}
				<span class="size-2 rounded-full bg-destructive" aria-hidden="true"></span>
				<span class="text-muted-foreground">Identity needs attention</span>
			{:else}
				<span class="size-2 animate-pulse rounded-full bg-warning" aria-hidden="true"></span>
				<span class="text-muted-foreground">Announcing…</span>
			{/if}
			<ChevronRight
				class="size-4 text-muted-foreground transition-transform duration-200 {expanderOpen
					? 'rotate-90'
					: ''}"
				aria-hidden="true"
			/>
		</button>
	</div>
</section>

<Collapsible.Root bind:open={expanderOpen}>
	<Collapsible.Content
		class="rounded-lg border border-border bg-card p-4 text-card-foreground data-[state=closed]:hidden"
	>
		<div class="mb-3 flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold">Your identity</h2>
			<span class="text-xs text-muted-foreground">{identityStore.statusMessage}</span>
		</div>
		<IdentityManager />
	</Collapsible.Content>
</Collapsible.Root>
