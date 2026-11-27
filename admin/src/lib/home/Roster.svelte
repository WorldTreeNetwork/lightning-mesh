<script lang="ts">
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, LOCAL, present } from './groups';
	import { groupSelection } from './selection.svelte';

	const view = $derived(
		present(
			groupSelection.groups,
			groupSelection.selectedId,
			heardRouters(groupResources(discovery.neighbors))
		)
	);

	const heardIds = $derived(new Set(view.count.heard.map((router) => router.id.toLowerCase())));
	const rows = $derived(
		view.selected.assigned.map((node) => ({
			...node,
			heard: heardIds.has(node.id.toLowerCase())
		}))
	);
</script>

<section class="roster" aria-label="Roster">
	<h2>{view.selected.name}</h2>
	{#if view.selected.id === LOCAL.id}
		<p class="quiet">Every node this computer can see is on Home. A guild roster is a membership list.</p>
	{:else if rows.length === 0}
		<p class="quiet">No routers are assigned to this group. Home lists whoever the last scan heard.</p>
	{:else}
		<ul>
			{#each rows as row (row.id)}
				<li class:quiet={!row.heard}>
					<span>{row.name}</span>
					<span>{row.heard ? 'Heard' : 'Not heard'}</span>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.roster {
		padding: var(--space-3);
	}

	h2 {
		margin: 0 0 var(--space-2);
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-medium);
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		justify-content: space-between;
		gap: var(--space-3);
		min-height: 28px;
		padding: var(--space-1) 0;
		border-bottom: 1px solid var(--border);
		font-size: var(--font-size-sm);
	}

	.quiet,
	li.quiet {
		color: var(--muted);
	}

	p {
		margin: 0;
		font-size: var(--font-size-sm);
	}
</style>
