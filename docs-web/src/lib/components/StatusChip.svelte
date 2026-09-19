<script lang="ts">
	let { status }: { status?: string } = $props();

	const label = $derived(
		status === 'built'
			? 'built'
			: status === 'partial'
				? 'partial'
				: status === 'coming-soon'
					? 'coming soon'
					: status
	);
</script>

{#if status}
	<span
		class="font-hud rounded-sm px-1.5 py-0.5 text-[0.65rem] tracking-wide uppercase"
		class:built={status === 'built'}
		class:partial={status === 'partial'}
		class:soon={status === 'coming-soon'}
	>
		{label}
	</span>
{/if}

<style>
	.built {
		color: var(--color-built);
		background: color-mix(in oklab, var(--color-built) 16%, transparent);
	}
	.partial {
		color: var(--color-partial);
		background: color-mix(in oklab, var(--color-partial) 16%, transparent);
	}
	.soon {
		color: var(--color-soon);
		background: color-mix(in oklab, var(--color-soon) 16%, transparent);
	}
</style>
