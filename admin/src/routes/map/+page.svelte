<script lang="ts">
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, LOCAL, present } from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';
	import { asRadioDocument, type RadioDocument } from '$lib/node/documents';
	import { fetchDocuments } from '$lib/node/fetch';
	import { layoutRadio, type RadioGraph } from '$lib/node/radio-graph';
	import { groupSubjects } from '$lib/node/subjects';

	const resources = $derived(groupResources(discovery.neighbors));
	const heard = $derived(heardRouters(resources));
	const view = $derived(present(groupSelection.groups, groupSelection.selectedId, heard));
	const subjects = $derived(
		groupSubjects(view.selected.id === LOCAL.id, resources, view.selected, heard)
	);

	let loading = $state(false);
	let graph = $state<RadioGraph>({ nodes: [], edges: [], paths: [] });
	let notes = $state<string[]>([]);

	$effect(() => {
		const requested = subjects;
		let cancelled = false;
		loading = true;
		notes = [];
		graph = { nodes: [], edges: [], paths: [] };
		if (requested.length === 0) {
			loading = false;
			return;
		}
		void (async () => {
			const rows = await fetchDocuments(requested, '/api/radio');
			if (cancelled) return;
			const problems: string[] = [];
			const input = rows.map((row) => {
				let radio: RadioDocument | null = null;
				if (row.document) {
					try {
						radio = asRadioDocument(row.document);
					} catch (cause) {
						problems.push(`${row.name}: ${cause instanceof Error ? cause.message : String(cause)}`);
					}
				} else if (row.error) {
					problems.push(`${row.name}: ${row.error}`);
				}
				return {
					id: row.id,
					name: row.name,
					quiet: !radio,
					mac: radio?.mesh_mac,
					stations: radio?.stations,
					mpaths: radio?.mpaths
				};
			});
			graph = layoutRadio(input);
			notes = problems;
			loading = false;
		})();
		return () => {
			cancelled = true;
		};
	});

	function at(id: string) {
		return graph.nodes.find((node) => node.id === id);
	}
</script>

<section class="page">
	<h1>Map</h1>
	<p>{view.selected.name}. Radio peers from the nodes that answered. This is not a floor plan.</p>
	{#if loading}<p role="status">Reading radios…</p>{/if}
	{#if !loading && subjects.length === 0}
		<p>No Lightning nodes on the last scan.</p>
	{/if}
	{#if graph.nodes.length > 0}
		<svg viewBox="0 0 640 420" role="img" aria-label="Radio graph">
			{#each graph.edges as edge (`${edge.from}-${edge.to}-${edge.quiet}`)}
				{@const from = at(edge.from)}
				{@const to = at(edge.to)}
				{#if from && to}
					<line
						x1={from.x}
						y1={from.y}
						x2={to.x}
						y2={to.y}
						class:quiet={edge.quiet}
					/>
					<text x={(from.x + to.x) / 2} y={(from.y + to.y) / 2 - 6}>{edge.label}</text>
				{/if}
			{/each}
			{#each graph.nodes as node (node.id)}
				<g class:quiet={node.quiet}>
					<circle cx={node.x} cy={node.y} r="16" />
					<text x={node.x} y={node.y + 32}>{node.name}</text>
				</g>
			{/each}
		</svg>
	{/if}
	{#if graph.paths.length > 0}
		<ul>
			{#each graph.paths as path (path)}
				<li>{path}</li>
			{/each}
		</ul>
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

	p,
	li {
		margin: var(--space-2) 0 0;
		color: var(--muted);
		font-size: var(--font-size-sm);
	}

	.error {
		color: var(--bad);
	}

	svg {
		width: 100%;
		max-width: 42rem;
		margin-top: var(--space-3);
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	line {
		stroke: var(--accent);
		stroke-width: 1.5;
	}

	line.quiet {
		stroke: var(--border);
		stroke-dasharray: 4 3;
	}

	circle {
		fill: var(--surface);
		stroke: var(--accent);
		stroke-width: 1.5;
	}

	g.quiet circle {
		stroke: var(--border);
	}

	text {
		fill: var(--muted);
		font-size: 11px;
		text-anchor: middle;
	}

	ul {
		margin: var(--space-3) 0 0;
		padding-left: 1.1rem;
	}
</style>
