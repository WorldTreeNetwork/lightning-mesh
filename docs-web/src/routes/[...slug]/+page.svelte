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
		{#if doc.meta.verified_against}
			<span class="text-[var(--color-ink-muted)]">verified {doc.meta.verified_against}</span>
		{/if}
	</p>
	{@html doc.body}
</article>

{#if data.next}
	<p class="mt-10 max-w-3xl text-sm">
		<a class="text-[var(--color-link)]" href="/{data.next.slug}">Next: {data.next.title} →</a>
	</p>
{/if}
