<script lang="ts">
	import type { GuildRecord } from './types';

	let { guild }: { guild: GuildRecord } = $props();
</script>

<section class="detail">
	<h3>{guild.name}</h3>

	<dl class="kv">
		<dt>profile</dt>
		<dd>{guild.profile}</dd>
		<dt>you</dt>
		<dd>{guild.you}</dd>
		<dt>id</dt>
		<dd><code class="addr">{guild.id}</code></dd>
		<dt>hex</dt>
		<dd><code class="addr">{guild.hex}</code></dd>
	</dl>

	<div class="block">
		<div class="label">Roster</div>
		{#if guild.roster.length === 0}
			<div class="empty">none</div>
		{:else}
			<table>
				<thead><tr><th>role</th><th>actor</th></tr></thead>
				<tbody>
					{#each guild.roster as m (m.actor)}
						<tr>
							<td>{m.role}</td>
							<td><code class="addr">{m.actor}</code></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>

	<div class="block">
		<div class="label">Nodes</div>
		{#if guild.nodes.length === 0}
			<div class="empty">none</div>
		{:else}
			<table>
				<thead><tr><th>name</th><th>node id</th><th>access id</th></tr></thead>
				<tbody>
					{#each guild.nodes as node, i (`${node.nodeId}-${i}`)}
						<tr>
							<td>{node.name}</td>
							<td><code class="addr">{node.nodeId}</code></td>
							<td><code class="addr">{node.accessId}</code></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>

	<div class="block">
		<div class="label">Ops</div>
		{#if guild.ops.length === 0}
			<div class="empty">none</div>
		{:else}
			<table>
				<thead><tr><th>n</th><th>kind</th><th>id</th></tr></thead>
				<tbody>
					{#each guild.ops as op (op.id)}
						<tr>
							<td>{op.n}</td>
							<td>{op.kind}</td>
							<td><code class="addr">{op.id}</code></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
</section>

<style>
	.detail {
		display: grid;
		gap: 0.85rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		min-width: 0;
	}

	h3 {
		margin: 0;
		font-size: 15px;
		color: var(--title-color);
		letter-spacing: var(--title-track);
	}

	.kv {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: 0.3rem 0.75rem;
		margin: 0;
	}

	dt,
	.label,
	th {
		color: var(--muted);
		font-size: 12px;
		font-weight: 600;
	}

	dd {
		margin: 0;
		min-width: 0;
	}

	.block {
		display: grid;
		gap: 0.3rem;
		min-width: 0;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		table-layout: auto;
		font-size: 13px;
	}

	th,
	td {
		padding: 0.3rem 0.5rem;
		border-bottom: 1px solid var(--border);
		text-align: left;
		vertical-align: top;
	}

	th:first-child,
	td:first-child {
		padding-left: 0;
		white-space: nowrap;
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
