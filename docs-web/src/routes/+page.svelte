<script lang="ts">
	import StatusChip from '$lib/components/StatusChip.svelte';

	let { data } = $props();
	const doc = $derived(data.doc);
</script>

<svelte:head>
	<title>{doc.title} · Lightning Mesh</title>
	{#if doc.meta.description}
		<meta name="description" content={doc.meta.description} />
	{/if}
</svelte:head>

<article class="prose prose-docs prose-invert max-w-3xl">
	<p class="font-hud mb-4 flex flex-wrap items-center gap-3 text-xs tracking-wide uppercase">
		<StatusChip status={doc.meta.status} />
		{#if doc.meta.time}<span class="text-[var(--color-ink-muted)]">{doc.meta.time}</span>{/if}
	</p>
	{@html doc.body}
</article>
