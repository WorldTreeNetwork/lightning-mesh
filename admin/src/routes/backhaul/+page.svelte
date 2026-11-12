<script lang="ts">
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, LOCAL, present } from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';
	import NetworkNameBar from '$lib/network-name/NetworkNameBar.svelte';
	import { asRadioDocument } from '$lib/node/documents';
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

	type RadioRow = {
		id: string;
		name: string;
		mesh: string;
		channel: string;
		frequency: string;
		peers: string;
	};

	let loading = $state(false);
	let rows = $state<RadioRow[]>([]);
	let notes = $state<string[]>([]);

	$effect(() => {
		const requested = subjects;
		let cancelled = false;
		loading = true;
		rows = [];
		notes = [];
		if (requested.length === 0) {
			loading = false;
			return;
		}
		void (async () => {
			const fetched = await fetchDocuments(requested, '/api/radio');
			if (cancelled) return;
			const next: RadioRow[] = [];
			const problems: string[] = [];
			for (const row of fetched) {
				if (!row.document) {
					if (row.error) problems.push(`${row.name}: ${row.error}`);
					continue;
				}
				try {
					const radio = asRadioDocument(row.document);
					next.push({
						id: row.id,
						name: row.name,
						mesh: radio.mesh_if ?? '—',
						channel: radio.channel === null ? '—' : String(radio.channel),
						frequency: radio.freq_mhz === null ? '—' : `${radio.freq_mhz} MHz`,
						peers: String(radio.stations.length)
					});
				} catch (cause) {
					problems.push(`${row.name}: ${cause instanceof Error ? cause.message : String(cause)}`);
				}
			}
			rows = next;
			notes = problems;
			loading = false;
		})();
		return () => {
			cancelled = true;
		};
	});
</script>

<section class="page">
	<h1>Backhaul</h1>
	<p>
		This is the radio mesh. A phone keeps its address as it moves between access points on it.
		The nodes sync that lease and follow the phone with a host route. Roaming is not a smaller
		group inside the mesh, and a guild does not decide who is on the radio.
	</p>
	{#if loading}<p class="note" role="status">Reading radios…</p>{/if}
	{#if !loading && rows.length === 0}
		<p class="note">No radio answered on the last scan.</p>
	{/if}
	{#if rows.length > 0}
		<table>
			<thead>
				<tr>
					<th>Node</th>
					<th>Mesh interface</th>
					<th>Channel</th>
					<th>Frequency</th>
					<th>Peers</th>
				</tr>
			</thead>
			<tbody>
				{#each rows as row (row.id)}
					<tr>
						<td>{row.name}</td>
						<td>{row.mesh}</td>
						<td>{row.channel}</td>
						<td>{row.frequency}</td>
						<td>{row.peers}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
	{#each notes as note (note)}
		<p class="note bad" role="alert">{note}</p>
	{/each}
	<NetworkNameBar />
</section>

<style>
	.page {
		display: flex;
		flex-direction: column;
		min-height: 100%;
	}

	h1 {
		margin: 0;
		padding: 0.8rem 1rem 0;
		font-size: 15px;
		font-weight: 600;
	}

	p {
		margin: 0.35rem 0 0.6rem;
		padding: 0 1rem;
		max-width: 42rem;
		color: var(--muted);
		font-size: 13px;
		line-height: 1.45;
	}

	.note {
		margin-top: 0;
	}

	.bad {
		color: var(--bad);
	}

	table {
		width: calc(100% - 2rem);
		margin: 0 1rem 0.8rem;
		border-collapse: collapse;
		font-size: 13px;
	}

	th,
	td {
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border);
		text-align: left;
	}

	th {
		color: var(--muted);
		font-weight: 600;
	}
</style>
