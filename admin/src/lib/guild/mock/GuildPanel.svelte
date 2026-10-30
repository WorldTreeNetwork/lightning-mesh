<script lang="ts">
	import { mockActor, mockGuilds } from './fixture';
	import type { GuildRecord } from './types';
	import IdentityCard from './IdentityCard.svelte';
	import GuildList from './GuildList.svelte';
	import GuildDetail from './GuildDetail.svelte';
	import GuildEditors from './GuildEditors.svelte';

	// Local copy so edits stay on screen. Never persisted, never signed.
	let guilds = $state<GuildRecord[]>(structuredClone(mockGuilds));

	const houses = $derived(guilds.filter((g) => g.outer === null));
	const fallbackId =
		mockGuilds.find((g) => g.workplace)?.id ?? mockGuilds.find((g) => g.outer === null)?.id ?? '';

	let selectedId = $state(fallbackId);

	const selected = $derived(guilds.find((g) => g.id === selectedId) ?? houses[0]);
	const outer = $derived(selected?.outer ? guilds.find((g) => g.id === selected.outer) : undefined);
	const listId = $derived(selected?.outer ?? selected?.id ?? '');

	const NODE_ID = /^[0-9a-f]{64}$/i;

	function onLocalName(name: string) {
		const g = guilds.find((x) => x.id === selected?.id);
		if (g) g.name = name;
	}

	function onAssign(nodeName: string, nodeId: string) {
		if (!NODE_ID.test(nodeId)) return;
		const g = guilds.find((x) => x.id === selected?.id);
		if (g) g.nodes.push({ name: nodeName, nodeId, accessId: 'pending' });
	}

	function openGuild(id: string) {
		if (guilds.some((g) => g.id === id)) selectedId = id;
	}
</script>

<div class="panel">
	<aside class="side">
		<IdentityCard actor={mockActor} />
		<GuildList guilds={guilds} selectedId={listId} onSelect={(id) => (selectedId = id)} />
	</aside>
	<div class="main">
		{#if selected}
			{#key selected.id}
				<GuildDetail
					guild={selected}
					outerName={outer?.name ?? null}
					onOpen={openGuild}
					onOpenOuter={() => outer && (selectedId = outer.id)}
				/>
				<GuildEditors guildName={selected.name} {onLocalName} {onAssign} />
			{/key}
		{:else}
			<div class="empty">No guild</div>
		{/if}
	</div>
</div>

<style>
	.panel {
		display: grid;
		grid-template-columns: minmax(16rem, 22rem) minmax(0, 1fr);
		gap: 0.75rem;
		padding: 0.75rem;
		background: var(--bg);
		color: var(--text);
		font-family: var(--font);
		border-bottom: 1px solid var(--border);
	}

	.side,
	.main {
		display: grid;
		align-content: start;
		gap: 0.75rem;
		min-width: 0;
	}

	.empty {
		color: var(--muted);
		font-size: 13px;
	}

	@media (max-width: 720px) {
		.panel {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
