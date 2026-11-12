<script lang="ts">
	import { onMount } from 'svelte';
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { guildSnapshot } from '$lib/guild/api';
	import { heardRouters, LOCAL, present } from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';

	let actor = $state('no seed');

	onMount(() => {
		void guildSnapshot()
			.then((snapshot) => {
				actor = snapshot.actor?.id ?? 'no seed';
			})
			.catch(() => {
				actor = 'no seed';
			});
	});

	const view = $derived(
		present(
			groupSelection.groups,
			groupSelection.selectedId,
			heardRouters(groupResources(discovery.neighbors))
		)
	);
</script>

<section class="page">
	<h1>You</h1>
	<div class="card">
		<p class="id">{actor}</p>
		{#if view.selected.id === LOCAL.id}
			<p>Local is this network, not a guild.</p>
		{:else}
			<p>Role in {view.selected.name}: {view.selected.you ?? 'unknown'}</p>
		{/if}
		<a href="/you/become">Become the admin</a>
	</div>
</section>

<style>
	.page {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-3);
		min-width: 0;
		padding: var(--space-5);
	}

	h1 {
		margin: 0;
		font-size: var(--font-size-lg);
		font-weight: var(--font-weight-medium);
	}

	.card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-3);
		width: 100%;
		min-width: 0;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: var(--space-4);
	}

	p {
		margin: 0;
		font-size: var(--font-size-sm);
	}

	.id {
		max-width: 100%;
		font-family: var(--mono);
		white-space: nowrap;
		overflow-x: auto;
	}

	a {
		min-height: 36px;
		display: inline-flex;
		align-items: center;
		padding: 0.3rem 0.75rem;
		border: 1px solid var(--primary-border);
		background: var(--primary-bg);
		color: var(--primary-fg);
		border-radius: var(--radius);
		text-decoration: none;
		font-size: var(--font-size-sm);
	}
</style>
