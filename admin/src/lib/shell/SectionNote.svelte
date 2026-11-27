<script lang="ts">
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, present } from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';

	let { title, body, backHref = '' }: { title: string; body: string; backHref?: string } = $props();

	const view = $derived(
		present(
			groupSelection.groups,
			groupSelection.selectedId,
			heardRouters(groupResources(discovery.neighbors))
		)
	);
</script>

<section class="page">
	{#if backHref}
		<a class="back" href={backHref}>Back</a>
	{/if}
	<h1>{title}</h1>
	<p>{view.selected.name}. {body}</p>
</section>

<style>
	.page {
		padding: var(--space-3);
	}

	.back {
		display: inline-block;
		margin-bottom: var(--space-2);
		color: var(--accent);
		font-size: var(--font-size-sm);
	}

	h1 {
		margin: 0;
		font-size: var(--font-size-lg);
		font-weight: var(--font-weight-medium);
	}

	p {
		margin: var(--space-2) 0 0;
		max-width: 42rem;
		color: var(--muted);
		font-size: var(--font-size-sm);
	}
</style>
