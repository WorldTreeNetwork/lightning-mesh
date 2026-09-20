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
		class="chip font-hud"
		class:built={status === 'built'}
		class:partial={status === 'partial'}
		class:soon={status === 'coming-soon'}
	>
		{label}
	</span>
{/if}

<style>
	.chip {
		display: inline-flex;
		align-items: center;
		border-radius: 999px;
		padding: 0.15rem 0.55rem;
		font-size: 0.62rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		line-height: 1.4;
		border: 1px solid color-mix(in oklab, currentColor 45%, transparent);
	}
	.built {
		color: var(--color-built);
		background: color-mix(in oklab, var(--color-built) 14%, transparent);
	}
	.partial {
		color: var(--color-partial);
		background: color-mix(in oklab, var(--color-partial) 16%, transparent);
	}
	.soon {
		color: #ffb3bd;
		background: color-mix(in oklab, var(--color-eva-red) 28%, transparent);
		border-color: color-mix(in oklab, var(--color-eva-red) 55%, transparent);
	}
</style>
