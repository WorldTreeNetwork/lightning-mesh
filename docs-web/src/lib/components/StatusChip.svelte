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
		padding: 0.18rem 0.6rem;
		font-size: 0.62rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		line-height: 1.4;
		border: 1px solid transparent;
		font-weight: 900;
	}
	.built {
		color: #101408;
		background: var(--color-built);
	}
	.partial {
		color: #1a0e00;
		background: var(--color-eva-orange);
	}
	.soon {
		color: #fff5f6;
		background: var(--color-eva-red);
	}
</style>
