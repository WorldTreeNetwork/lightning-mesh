<script lang="ts">
	import { NETWORK_NAME_LABEL, networkName } from './store.svelte';
</script>

<div class="nn-bar">
	<label for="network-name">{NETWORK_NAME_LABEL}</label>
	<input
		id="network-name"
		type="text"
		autocomplete="off"
		spellcheck="false"
		bind:value={networkName.name}
		disabled={networkName.applying}
		onkeydown={(e) => {
			if (e.key === 'Enter' && networkName.canApply) void networkName.apply();
		}}
	/>
	<button
		type="button"
		class="apply"
		disabled={!networkName.canApply}
		onclick={() => networkName.apply()}
	>
		{networkName.applying ? 'Applying…' : 'Apply'}
	</button>
	{#if networkName.applying}
		<span class="nn-status">Applying across the reachable nodes…</span>
	{:else if networkName.error}
		<span class="nn-status bad">{networkName.error}</span>
	{:else if networkName.report}
		<span class="nn-status" class:bad={Boolean(networkName.report.halted)}>
			{#if networkName.report.halted}
				Halted at {networkName.report.halted}
			{:else}
				Updated {networkName.report.updated.length
					? networkName.report.updated.join(', ')
					: 'none'}
			{/if}
			{#if networkName.report.skipped.length}
				· skipped {networkName.report.skipped.join(', ')}
			{/if}
		</span>
	{/if}
	{#if networkName.report?.log}
		<details class="nn-log">
			<summary>apply log</summary>
			<pre>{networkName.report.log}</pre>
		</details>
	{/if}
</div>

<style>
	.nn-bar {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.55rem 1rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
		flex-wrap: wrap;
	}

	.nn-bar label {
		font-size: 13px;
		font-weight: 600;
		color: var(--text);
		white-space: nowrap;
		letter-spacing: var(--track);
		text-transform: var(--case);
	}

	.nn-bar input {
		flex: 1;
		min-width: 12rem;
		min-height: 36px;
		background: var(--bg);
		border: 1px solid var(--border-strong);
		color: var(--text);
		border-radius: var(--radius);
		padding: 0.3rem 0.55rem;
	}

	.nn-bar input:focus {
		outline: 2px solid var(--focus);
		outline-offset: 1px;
	}

	.apply {
		min-height: 36px;
		border: 1px solid var(--primary-border);
		background: var(--primary-bg);
		color: var(--primary-fg);
		padding: 0.3rem 0.75rem;
		border-radius: var(--radius);
		font-size: 13px;
	}

	.apply:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	.nn-status {
		color: var(--muted);
		font-size: 13px;
	}

	.nn-status.bad {
		color: var(--bad);
	}

	.nn-log {
		flex-basis: 100%;
		color: var(--muted);
		font-size: 13px;
	}

	.nn-log summary {
		cursor: pointer;
	}

	.nn-log pre {
		margin: 0.35rem 0 0;
		max-height: 8rem;
		overflow: auto;
		padding: 0.5rem 0.65rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface-2);
		color: var(--text);
		font-family: var(--mono);
		font-size: 12px;
		white-space: pre-wrap;
	}
</style>
