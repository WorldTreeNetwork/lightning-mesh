<script lang="ts">
	import { Monitor, Router } from '@lucide/svelte';
	import Radar from '$lib/discovery/Radar.svelte';
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import {
		heardRouters,
		houseTiles,
		LOCAL,
		localTiles,
		needsAdmin,
		present,
		selectionCount
	} from '$lib/home/groups';
	import { groupSelection } from '$lib/home/selection.svelte';

	const resources = $derived(groupResources(discovery.neighbors));
	const heard = $derived(heardRouters(resources));
	const local = $derived(localTiles(resources));
	const view = $derived(present(groupSelection.groups, groupSelection.selectedId, heard, local));
	const guildTiles = $derived(houseTiles(view.selected, heard));
	const onLocal = $derived(view.selected.id === LOCAL.id);

	const hint = $derived.by(() => {
		if (discovery.loading) return 'Scanning';
		if (discovery.error) return 'Scan failed';
		if (onLocal) return local.length === 0 ? 'Nothing on this network' : 'on this network';
		const { active, total } = view.count;
		if (total === null) return active === 0 ? 'Nothing on this scan' : 'routers heard';
		if (active === 0) return 'None answered';
		if (active === total) return 'All answered';
		return 'routers heard';
	});
</script>

<section class="home">
	<div class="lead">
		<p class="count">{discovery.loading ? '…' : selectionCount(view.selected, view.count)}</p>
		<p class="hint">{hint}</p>
		<span class="scan">
			<Radar scanning={discovery.loading} onclick={() => void discovery.scan()} />
		</span>
	</div>
	{#if !onLocal && needsAdmin(view.selected.you)}
		<p class="next"><a href="/you">You are not an admin of this group.</a></p>
	{/if}
	{#if discovery.error}
		<p class="error" role="alert">{discovery.error}</p>
	{/if}
	{#if onLocal}
		{#if local.length > 0}
			<ul class="house" aria-label="Nodes on this network">
				{#each local as tile (tile.id + tile.detail)}
					<li>
						<a class="router" href={tile.href}>
							{#if tile.href.startsWith('/router/')}
								<Router size={22} strokeWidth={1.75} aria-hidden="true" />
							{:else}
								<Monitor size={22} strokeWidth={1.75} aria-hidden="true" />
							{/if}
							<span class="name">{tile.name}</span>
							{#if tile.detail !== tile.name}
								<span class="state">{tile.detail}</span>
							{/if}
						</a>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="quiet">Nothing answered on this network yet.</p>
		{/if}
	{:else if guildTiles.length > 0}
		<ul class="house" aria-label="Routers in {view.selected.name}">
			{#each guildTiles as tile (tile.id)}
				<li>
					<a class="router" class:quiet={!tile.heard} href="/router/{tile.id}">
						<Router size={22} strokeWidth={1.75} aria-hidden="true" />
						<span class="name">{tile.name}</span>
						<span class="state">{tile.heard ? 'Heard' : 'Not heard'}</span>
					</a>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="quiet">
			No routers are assigned to this group.
			<a href="/guild">Assign one from Guild.</a>
		</p>
	{/if}
</section>

<style>
	.home {
		padding: var(--space-5);
	}

	.lead {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin: 0 0 var(--space-4);
	}

	.scan {
		margin-left: auto;
		display: inline-flex;
	}

	.count,
	.hint,
	.quiet,
	.next,
	.error {
		margin: 0;
	}

	.count {
		font-size: var(--font-size-2xl);
		font-weight: var(--font-weight-medium);
		line-height: var(--line-height-tight);
		letter-spacing: var(--letter-spacing-tight);
		font-variant-numeric: tabular-nums;
	}

	.hint,
	.quiet,
	.state {
		color: var(--muted);
		font-size: var(--font-size-sm);
	}

	.error {
		margin-top: var(--space-2);
		color: var(--bad);
		font-size: var(--font-size-sm);
	}

	.next {
		margin: 0 0 var(--space-4);
		font-size: var(--font-size-sm);
	}

	.next a,
	.quiet a {
		color: var(--accent);
	}

	.house {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.router {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-2);
		width: 220px;
		min-height: 112px;
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		color: var(--text);
		text-decoration: none;
	}

	.router.quiet {
		background: transparent;
		color: var(--muted);
	}

	.router:hover {
		background: var(--row-hover);
		color: var(--text);
	}

	.router:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.name,
	.state {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.name {
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-medium);
	}

	.router:not(.quiet) .state {
		color: var(--ok);
	}
</style>
