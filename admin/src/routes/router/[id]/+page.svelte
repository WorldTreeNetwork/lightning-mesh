<script lang="ts">
	import { page } from '$app/state';
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, present, routerTitle } from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';
	import { asNodeDocument, asRadioDocument, type NodeDocument, type RadioDocument } from '$lib/node/documents';
	import { fetchLanJson } from '$lib/node/fetch';
	import { lanTarget, resourceForMesh } from '$lib/node/lan';

	const resources = $derived(groupResources(discovery.neighbors));
	const heard = $derived(heardRouters(resources));
	const view = $derived(present(groupSelection.groups, groupSelection.selectedId, heard));
	const title = $derived(routerTitle(page.params.id ?? '', view.selected, heard));

	let loading = $state(false);
	let error = $state('');
	let node = $state<NodeDocument | null>(null);
	let radio = $state<RadioDocument | null>(null);

	$effect(() => {
		const id = page.params.id ?? '';
		const resource = resourceForMesh(resources, id);
		const target = resource ? lanTarget(resource.rows) : null;
		let cancelled = false;
		loading = true;
		error = '';
		node = null;
		radio = null;
		if (!target) {
			loading = false;
			error = resource
				? 'No LAN address for this router on the last scan.'
				: 'This router is not on the last scan.';
			return;
		}
		void (async () => {
			const [nodeResult, radioResult] = await Promise.allSettled([
				fetchLanJson(target.address, '/api/node', target.scopeId),
				fetchLanJson(target.address, '/api/radio', target.scopeId)
			]);
			if (cancelled) return;
			const problems: string[] = [];
			if (nodeResult.status === 'fulfilled') {
				try {
					node = asNodeDocument(nodeResult.value);
				} catch (cause) {
					problems.push(cause instanceof Error ? cause.message : String(cause));
				}
			} else {
				problems.push(
					nodeResult.reason instanceof Error ? nodeResult.reason.message : String(nodeResult.reason)
				);
			}
			if (radioResult.status === 'fulfilled') {
				try {
					radio = asRadioDocument(radioResult.value);
				} catch (cause) {
					problems.push(cause instanceof Error ? cause.message : String(cause));
				}
			} else {
				problems.push(
					radioResult.reason instanceof Error ? radioResult.reason.message : String(radioResult.reason)
				);
			}
			error = problems.join(' ');
			loading = false;
		})();
		return () => {
			cancelled = true;
		};
	});

	const facts = $derived.by(() => {
		const rows: { label: string; value: string }[] = [];
		const put = (label: string, value: string | number | null | undefined) => {
			if (value === null || value === undefined || value === '') return;
			rows.push({ label, value: String(value) });
		};
		put('Node id', node?.node_id);
		put('Subnet', node?.subnet);
		put('Backhaul', node?.backhaul_addr ?? radio?.backhaul_addr);
		put('Mesh interface', radio?.mesh_if);
		put('Channel', radio?.channel);
		put('Frequency', radio?.freq_mhz === null || radio?.freq_mhz === undefined ? null : `${radio.freq_mhz} MHz`);
		put('Mesh MAC', radio?.mesh_mac);
		return rows;
	});
</script>

<section class="page">
	<a class="back" href="/">Home</a>
	<h1>{title}</h1>
	{#if loading}<p role="status">Reading this router…</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#if facts.length > 0}
		<dl>
			{#each facts as fact (fact.label)}
				<div>
					<dt>{fact.label}</dt>
					<dd>{fact.value}</dd>
				</div>
			{/each}
		</dl>
	{/if}
	{#if radio}
		<h2>Stations</h2>
		{#if radio.stations.length === 0}
			<p>No peer stations.</p>
		{:else}
			<table>
				<thead>
					<tr>
						<th>MAC</th>
						<th>Signal</th>
						<th>Throughput</th>
						<th>Idle</th>
					</tr>
				</thead>
				<tbody>
					{#each radio.stations as station (station.mac)}
						<tr>
							<td>{station.mac}</td>
							<td>{station.signal_dbm === null ? '—' : `${station.signal_dbm} dBm`}</td>
							<td>{station.expected_throughput_mbps === null ? '—' : `${station.expected_throughput_mbps} Mb/s`}</td>
							<td>{station.inactive_ms === null ? '—' : `${station.inactive_ms} ms`}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
		<h2>Paths</h2>
		{#if radio.mpaths.length === 0}
			<p>No mesh paths.</p>
		{:else}
			<table>
				<thead>
					<tr>
						<th>Destination</th>
						<th>Next hop</th>
						<th>Metric</th>
					</tr>
				</thead>
				<tbody>
					{#each radio.mpaths as path (`${path.dst}-${path.next_hop}`)}
						<tr>
							<td>{path.dst}</td>
							<td>{path.next_hop}</td>
							<td>{path.metric ?? '—'}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
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

	h1,
	h2 {
		margin: var(--space-2) 0 0;
		font-weight: var(--font-weight-medium);
	}

	h1 {
		font-size: var(--font-size-lg);
	}

	h2 {
		font-size: var(--font-size-sm);
	}

	p,
	dd {
		margin: var(--space-2) 0 0;
		color: var(--muted);
		font-size: var(--font-size-sm);
	}

	.error {
		color: var(--bad);
	}

	dl {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
		gap: var(--space-2) var(--space-4);
		margin: var(--space-3) 0 0;
	}

	dt {
		color: var(--muted);
		font-size: 12px;
	}

	dd {
		margin: 0;
		color: var(--text);
	}

	table {
		width: 100%;
		margin-top: var(--space-2);
		border-collapse: collapse;
		font-size: var(--font-size-sm);
	}

	th,
	td {
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border);
		text-align: left;
	}

	th {
		color: var(--muted);
		font-weight: var(--font-weight-medium);
	}
</style>
