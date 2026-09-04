<script lang="ts">
	import { copyText } from '$lib/clipboard';
	import type { LinkLocalNeighbor } from './api';
	import { ipv6ToBase58 } from './base58';

	let { neighbor }: { neighbor: LinkLocalNeighbor } = $props();

	const encoded = $derived(neighbor.base58 || ipv6ToBase58(neighbor.address) || neighbor.address);

	let copied = $state<string | null>(null);
	let timer: ReturnType<typeof setTimeout> | null = null;

	async function copy(which: 'base58' | 'ipv6', text: string) {
		const ok = await copyText(text);
		if (timer) clearTimeout(timer);
		copied = ok ? which : 'fail';
		timer = setTimeout(() => {
			copied = null;
		}, 1400);
	}
</script>

<div class="cell">
	<button
		type="button"
		class="b58"
		title="Copy base58"
		onclick={() => copy('base58', encoded)}
	>
		{encoded}
	</button>
	<div class="pop" role="tooltip">
		<div class="pop-row">
			<span class="k">base58</span>
			<code class="v">{encoded}</code>
			<button type="button" class="copy" onclick={() => copy('base58', encoded)}>copy</button>
		</div>
		<div class="pop-row">
			<span class="k">ipv6</span>
			<code class="v">{neighbor.address}</code>
			<button type="button" class="copy" onclick={() => copy('ipv6', neighbor.address)}>copy</button>
		</div>
		{#if neighbor.scoped !== neighbor.address}
			<div class="pop-row">
				<span class="k">zone</span>
				<code class="v">{neighbor.scoped}</code>
				<button type="button" class="copy" onclick={() => copy('ipv6', neighbor.scoped)}>copy</button>
			</div>
		{/if}
		<div class="hint">
			{#if copied === 'base58'}
				copied base58
			{:else if copied === 'ipv6'}
				copied ipv6
			{:else if copied === 'fail'}
				copy failed
			{:else}
				hover to read · click to copy
			{/if}
		</div>
	</div>
</div>

<style>
	.cell {
		position: relative;
		display: inline-block;
		max-width: 100%;
		z-index: 1;
	}

	.cell:hover,
	.cell:focus-within {
		z-index: 8;
	}

	.b58 {
		border: 0;
		background: transparent;
		color: var(--mint);
		padding: 0;
		font: inherit;
		font-size: 12.5px;
		letter-spacing: 0.02em;
		text-align: left;
	}

	.b58:hover,
	.b58:focus-visible {
		text-decoration: underline;
		text-underline-offset: 3px;
	}

	.pop {
		display: none;
		position: absolute;
		left: 0;
		top: 100%;
		z-index: 20;
		min-width: 22rem;
		max-width: min(38rem, 70vw);
		padding: 0.55rem 0.65rem 0.45rem;
		border: 1px solid var(--line-strong);
		background: color-mix(in srgb, var(--panel-2) 94%, black);
		box-shadow: 0 10px 28px #00000088;
	}

	.cell:hover .pop,
	.cell:focus-within .pop {
		display: block;
	}

	.pop-row {
		display: grid;
		grid-template-columns: 4.2rem 1fr auto;
		gap: 0.45rem;
		align-items: center;
		margin-bottom: 0.28rem;
	}

	.k {
		color: var(--muted);
		font-size: 10px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.v {
		font: inherit;
		font-size: 12px;
		color: var(--text);
		user-select: all;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.copy {
		border: 1px solid var(--line-strong);
		background: #0e2a22;
		color: var(--mint);
		padding: 0.08rem 0.4rem;
		font-size: 10px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.copy:hover {
		border-color: var(--mint);
	}

	.hint {
		margin-top: 0.2rem;
		color: var(--muted);
		font-size: 10px;
		letter-spacing: 0.06em;
	}
</style>
