<script lang="ts">
	import type { GuildRecord } from './types';

	let {
		guilds,
		selectedId,
		onSelect
	}: { guilds: GuildRecord[]; selectedId: string; onSelect: (id: string) => void } = $props();

	const houses = $derived(guilds.filter((g) => g.outer === null));
	const nested = $derived(guilds.filter((g) => g.outer !== null));
</script>

<section class="list">
	<div class="label">Guilds</div>
	{#each houses as guild (guild.id)}
		<button
			type="button"
			class="row"
			class:on={guild.id === selectedId}
			aria-pressed={guild.id === selectedId}
			onclick={() => onSelect(guild.id)}
		>
			<span class="top">
				<span class="name">{guild.name}</span>
				<span class="kind">{guild.workplace ? 'workplace' : 'house'}</span>
			</span>
			<span class="meta">
				<span>you {guild.you}</span>
				<span>{guild.nodes.length} {guild.nodes.length === 1 ? 'node' : 'nodes'}</span>
			</span>
			<code class="addr">{guild.id}</code>
		</button>
	{:else}
		<div class="empty">none</div>
	{/each}
	{#if nested.length}
		<div class="label">Nested</div>
		{#each nested as guild (guild.id)}
			<button
				type="button"
				class="row"
				class:on={guild.id === selectedId}
				aria-pressed={guild.id === selectedId}
				onclick={() => onSelect(guild.id)}
			>
				<span class="top">
					<span class="name">{guild.name}</span>
					<span class="kind">{guild.profile}</span>
				</span>
				<span class="meta">
					<span>you {guild.you}</span>
				</span>
				<code class="addr">{guild.id}</code>
			</button>
		{/each}
	{/if}
</section>

<style>
	.list {
		display: grid;
		gap: 0.35rem;
	}

	.label {
		color: var(--muted);
		font-size: 12px;
		font-weight: 600;
	}

	.row {
		display: grid;
		gap: 0.2rem;
		width: 100%;
		padding: 0.5rem 0.65rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		color: var(--text);
		font-size: 13px;
		text-align: left;
		letter-spacing: 0;
		text-transform: none;
	}

	.row:hover:not(.on) {
		background: var(--row-hover);
	}

	.row.on {
		background: var(--accent-soft);
		border-color: var(--accent);
	}

	.top,
	.meta {
		display: flex;
		gap: 0.6rem;
		align-items: baseline;
	}

	.name {
		font-weight: 600;
	}

	.kind,
	.meta {
		color: var(--muted);
		font-size: 12px;
	}

	.addr {
		font-family: var(--mono);
		font-size: 12px;
		overflow-wrap: anywhere;
		user-select: text;
		cursor: text;
	}

	.empty {
		color: var(--muted);
		font-size: 13px;
	}
</style>
