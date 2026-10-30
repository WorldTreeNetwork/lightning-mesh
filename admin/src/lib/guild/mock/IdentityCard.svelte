<script lang="ts">
	import type { GuildActor } from './types';

	let { actor }: { actor: GuildActor } = $props();
</script>

<section class="card">
	<div class="head">
		<span class="label">Identity</span>
		<span class="you">you</span>
	</div>
	<code class="addr">{actor.id}</code>
</section>

<style>
	.card {
		display: grid;
		gap: 0.35rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.label {
		color: var(--muted);
		font-size: 12px;
		font-weight: 600;
	}

	.you {
		padding: 0 0.4rem;
		border-radius: var(--radius);
		background: var(--accent-soft);
		color: var(--accent);
		font-size: 12px;
	}

	.addr {
		font-family: var(--mono);
		font-size: 12px;
		overflow-wrap: anywhere;
		user-select: all;
	}
</style>
