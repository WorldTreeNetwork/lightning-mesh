<script lang="ts">
	let { title, backHref = '' }: { title: string; backHref?: string } = $props();
</script>

<section class="page">
	{#if backHref}
		<a class="back" href={backHref}>Back</a>
	{/if}
	<h1>{title}</h1>
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
		line-height: var(--line-height-tight);
		letter-spacing: var(--letter-spacing-tight);
	}
</style>
