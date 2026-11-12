<script lang="ts">
	import { page } from '$app/state';
	import { copyText } from '$lib/clipboard';
	import { formatVia, groupResources, resourceLabel } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';

	const resource = $derived(
		groupResources(discovery.neighbors).find((item) => item.id === (page.params.id ?? ''))
	);
	let copied = $state('');

	async function copyAddress(address: string) {
		copied = (await copyText(address)) ? address : '';
	}
</script>

<section class="page">
	<a class="back" href="/network">Network</a>
	{#if resource}
		<h1>{resourceLabel(resource)}</h1>
		<p class="meta">
			{resource.state}
			{#if resource.via.length}
				· via {formatVia(resource.via)}
			{/if}
		</p>
		<table>
			<thead>
				<tr>
					<th>Interface</th>
					<th>Address</th>
					<th>State</th>
				</tr>
			</thead>
			<tbody>
				{#each resource.rows as row (`${row.iface}-${row.address}`)}
					<tr>
						<td>{row.iface}</td>
						<td>
							<button type="button" onclick={() => copyAddress(row.address)}>
								{row.address}
							</button>
							{#if copied === row.address}<span class="copied">Copied</span>{/if}
						</td>
						<td>{row.state}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{:else}
		<h1>Host</h1>
		<p>This host is not on the last scan.</p>
	{/if}
</section>

<style>
	.page {
		padding: var(--space-3);
	}

	.back {
		color: var(--accent);
		font-size: var(--font-size-sm);
	}

	h1 {
		margin: var(--space-2) 0 0;
		font-size: var(--font-size-lg);
		font-weight: var(--font-weight-medium);
	}

	.meta,
	p {
		margin: var(--space-2) 0 0;
		color: var(--muted);
		font-size: var(--font-size-sm);
	}

	table {
		width: 100%;
		margin-top: var(--space-3);
		border-collapse: collapse;
		font-size: var(--font-size-sm);
	}

	th,
	td {
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border);
		text-align: left;
		vertical-align: baseline;
	}

	th {
		color: var(--muted);
		font-weight: var(--font-weight-medium);
	}

	button {
		padding: 0;
		border: 0;
		background: none;
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	button:hover {
		color: var(--accent);
	}

	.copied {
		margin-left: 0.4rem;
		color: var(--muted);
	}
</style>
