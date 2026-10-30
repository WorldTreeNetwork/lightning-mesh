<script lang="ts">
	import RoleBadge from '../RoleBadge.svelte';
	import type { GuildRecord } from './types';

	let {
		guild,
		outerName = null,
		onOpen,
		onOpenOuter
	}: {
		guild: GuildRecord;
		outerName?: string | null;
		onOpen: (id: string) => void;
		onOpenOuter: () => void;
	} = $props();

</script>

<section class="detail">
	<div class="head">
		<h3>{guild.name}</h3>
		<span class="meta">{guild.workplace ? 'workplace' : guild.profile}</span>
		<RoleBadge role={guild.you} />
	</div>
	{#if outerName}
		<button type="button" class="back" onclick={onOpenOuter}>In {outerName}</button>
	{/if}

	<div class="block">
		<div class="label">Nodes</div>
		{#if guild.nodes.length === 0}
			<div class="empty">none</div>
		{:else}
			{#each guild.nodes as node, i (`${node.nodeId}-${i}`)}
				<details>
					<summary>{node.name}</summary>
					<dl class="kv">
						<dt>node id</dt>
						<dd><code class="addr">{node.nodeId}</code></dd>
						<dt>access</dt>
						<dd><code class="addr">{node.accessId}</code></dd>
					</dl>
					{#if node.accessId !== 'pending'}
						<button type="button" class="open" onclick={() => onOpen(node.accessId)}>Open access guild</button>
					{/if}
				</details>
			{/each}
		{/if}
	</div>

	<details>
		<summary>Roster <span class="count">{guild.roster.length}</span></summary>
		{#if guild.roster.length === 0}
			<div class="empty">none</div>
		{:else}
			<ul>
				{#each guild.roster as m (m.actor)}
					<li><RoleBadge role={m.role} /> <code class="addr">{m.actor}</code></li>
				{/each}
			</ul>
		{/if}
	</details>

	<details>
		<summary>Ops <span class="count">{guild.ops.length}</span></summary>
		{#if guild.ops.length === 0}
			<div class="empty">none</div>
		{:else}
			<ol>
				{#each guild.ops as op (`${op.n}-${op.kind}`)}
					<li><span class="n">{op.n}</span> {op.kind} <code class="addr">{op.id}</code></li>
				{/each}
			</ol>
		{/if}
	</details>

	<details>
		<summary>Id</summary>
		<dl class="kv">
			<dt>id</dt>
			<dd><code class="addr">{guild.id}</code></dd>
			<dt>hex</dt>
			<dd><code class="addr">{guild.hex}</code></dd>
		</dl>
	</details>
</section>

<style>
	.detail {
		display: grid;
		gap: 0.65rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		min-width: 0;
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem 0.75rem;
		align-items: baseline;
	}

	h3 {
		margin: 0;
		font-size: 15px;
		color: var(--title-color);
		letter-spacing: var(--title-track);
	}

	.meta,
	.count,
	.label,
	dt {
		color: var(--muted);
		font-size: 12px;
		font-weight: 600;
	}

	.count {
		font-weight: 500;
	}

	.back,
	.open {
		justify-self: start;
		font: inherit;
		font-size: 13px;
		padding: 4px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface-2);
		color: var(--text);
		cursor: pointer;
	}

	.block {
		display: grid;
		gap: 0.3rem;
	}

	details {
		border-top: 1px solid var(--border);
		padding-top: 0.4rem;
	}

	.block details {
		border-top: 0;
		padding-top: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.35rem 0.5rem;
		background: var(--surface-2);
	}

	summary {
		cursor: pointer;
		font-size: 13px;
		font-weight: 600;
	}

	.kv {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: 0.3rem 0.75rem;
		margin: 0.45rem 0 0;
	}

	dd {
		margin: 0;
		min-width: 0;
	}

	ul,
	ol {
		margin: 0.4rem 0 0;
		padding: 0;
		list-style: none;
		display: grid;
		gap: 0.35rem;
	}

	li {
		font-size: 13px;
		min-width: 0;
	}

	.n {
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}

	.addr {
		font-family: var(--mono);
		font-size: 12px;
		overflow-wrap: anywhere;
		word-break: break-all;
		user-select: all;
	}

	.empty {
		color: var(--muted);
		font-size: 13px;
	}
</style>
