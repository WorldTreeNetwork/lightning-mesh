<script lang="ts">
	import { onMount } from 'svelte';
	import { guildSnapshot, guildRename, guildAssign } from '../api';
	import type { GuildActor, GuildRecord } from './types';
	import IdentityCard from './IdentityCard.svelte';
	import GuildList from './GuildList.svelte';
	import GuildDetail from './GuildDetail.svelte';
	import GuildEditors from './GuildEditors.svelte';

	let guilds = $state<GuildRecord[]>([]);
	let actor = $state<GuildActor>({ id: 'no seed' });
	let loading = $state(false);
	let error = $state('');
	let selectedId = $state('');

	const houses = $derived(guilds.filter((g) => g.outer === null));

	onMount(() => { void reload(); });

	async function reload() {
		loading = true;
		error = '';
		try {
			const snapshot = await guildSnapshot();
			const ids = new Map(snapshot.guilds.map((g) => [g.keyspaceId, g.id]));
			actor = { id: snapshot.actor?.id ?? 'no seed' };
			guilds = snapshot.guilds.map((g): GuildRecord => ({
				name: g.name ?? (g.profile === 'access-node' ? 'access-node' : 'guild'),
				profile: g.profile === 'access-node' ? 'access-node' : 'house',
				workplace: g.workplace,
				you: g.you,
				id: g.id,
				hex: g.keyspaceId,
				outer: g.profile === 'access-node' && g.outer ? ids.get(g.outer) ?? null : null,
				roster: g.roster.map((member) => ({
					role: member.admin ? 'admin' : 'member', actor: member.actor
				})),
				nodes: g.nodes.map((node) => ({
					name: node.name ?? 'node', nodeId: node.nodeId, accessId: node.accessId ?? 'pending'
				})),
				ops: snapshot.ops.filter((op) => op.about === g.keyspaceId || op.hex === g.keyspaceId)
			}));
			if (!guilds.some((g) => g.id === selectedId)) {
				selectedId = guilds.find((g) => g.workplace)?.id ?? guilds.find((g) => g.outer === null)?.id ?? '';
			}
		} catch (cause) {
			error = String(cause);
		} finally {
			loading = false;
		}
	}

	const selected = $derived(guilds.find((g) => g.id === selectedId) ?? houses[0]);
	const outer = $derived(selected?.outer ? guilds.find((g) => g.id === selected.outer) : undefined);
	const listId = $derived(selected?.outer ?? selected?.id ?? '');

	const NODE_ID = /^[0-9a-f]{64}$/i;

	async function onLocalName(name: string) {
		if (!selected || loading) return;
		loading = true;
		error = '';
		try {
			await guildRename(selected.hex, name);
			await reload();
		} catch (cause) {
			error = String(cause);
		} finally {
			loading = false;
		}
	}

	async function onAssign(nodeName: string, nodeId: string) {
		if (!selected || loading || !NODE_ID.test(nodeId)) return;
		loading = true;
		error = '';
		try {
			await guildAssign(selected.hex, nodeName, nodeId);
			await reload();
		} catch (cause) {
			error = String(cause);
		} finally {
			loading = false;
		}
	}

	function openGuild(id: string) {
		if (guilds.some((g) => g.id === id)) selectedId = id;
	}
</script>

<div class="panel">
	<aside class="side">
		<IdentityCard {actor} />
		<GuildList guilds={guilds} selectedId={listId} onSelect={(id) => (selectedId = id)} />
	</aside>
	<div class="main">
		{#if loading}<div class="empty" role="status">Loading…</div>{/if}
		{#if error}<div class="empty" role="alert">{error}</div>{/if}
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
