<script lang="ts">
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, LOCAL, present } from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';
	import { asDirectoryDocument, mergeServices, type DirectoryService, type ServiceRow } from '$lib/node/documents';
	import { fetchDocuments } from '$lib/node/fetch';
	import { groupSubjects } from '$lib/node/subjects';

	const resources = $derived(groupResources(discovery.neighbors));
	const heard = $derived(heardRouters(resources));
	const view = $derived(present(groupSelection.groups, groupSelection.selectedId, heard));
	const subjects = $derived(
		groupSubjects(view.selected.id === LOCAL.id, resources, view.selected, heard).filter(
			(subject) => subject.heard
		)
	);

	let loading = $state(false);
	let rows = $state<ServiceRow[]>([]);
	let notes = $state<string[]>([]);
	let asked = $state(false);

	$effect(() => {
		const requested = subjects;
		let cancelled = false;
		loading = true;
		rows = [];
		notes = [];
		asked = requested.length > 0;
		if (requested.length === 0) {
			loading = false;
			return;
		}
		void (async () => {
			const fetched = await fetchDocuments(requested, '/api/directory');
			if (cancelled) return;
			const reports: { node: string; services: DirectoryService[] }[] = [];
			const problems: string[] = [];
			for (const row of fetched) {
				if (!row.document) {
					if (row.error) problems.push(`${row.name}: ${row.error}`);
					continue;
				}
				try {
					reports.push({ node: row.name, services: asDirectoryDocument(row.document).services });
				} catch (cause) {
					problems.push(`${row.name}: ${cause instanceof Error ? cause.message : String(cause)}`);
				}
			}
			rows = mergeServices(reports);
			notes = problems;
			loading = false;
		})();
		return () => {
			cancelled = true;
		};
	});
</script>

<section class="page">
	<h1>Services</h1>
	<p>{view.selected.name}. Published services from each Lightning node that answered.</p>
	{#if loading}<p role="status">Reading directories…</p>{/if}
	{#if !loading && !asked}
		<p>No Lightning nodes on the last scan.</p>
	{:else if !loading && asked && rows.length === 0 && notes.length === 0}
		<p>No services are published on this network.</p>
	{/if}
	{#if rows.length > 0}
		<table>
			<thead>
				<tr>
					<th>Name</th>
					<th>Address</th>
					<th>Port</th>
					<th>Protocol</th>
					<th>Reported by</th>
				</tr>
			</thead>
			<tbody>
				{#each rows as row (`${row.name}-${row.ip}-${row.port}`)}
					<tr>
						<td>{row.name}</td>
						<td>{row.ip}</td>
						<td>{row.port}</td>
						<td>{row.protocol || '—'}</td>
						<td>{row.reportedBy.join(', ')}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
	{#each notes as note (note)}
		<p class="error" role="alert">{note}</p>
	{/each}
</section>

<style>
	.page {
		padding: var(--space-3);
	}

	h1 {
		margin: 0;
		font-size: var(--font-size-lg);
		font-weight: var(--font-weight-medium);
	}

	p {
		margin: var(--space-2) 0 0;
		color: var(--muted);
		font-size: var(--font-size-sm);
	}

	.error {
		color: var(--bad);
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
	}

	th {
		color: var(--muted);
		font-weight: var(--font-weight-medium);
	}
</style>
