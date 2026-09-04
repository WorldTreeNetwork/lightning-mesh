<script lang="ts">
	import { onMount } from 'svelte';
	import { copyText } from '$lib/clipboard';
	import AddrCell from '$lib/discovery/AddrCell.svelte';
	import { discovery } from '$lib/discovery/store.svelte';
	import { NETWORK_NAME_LABEL, networkName } from '$lib/network-name/store.svelte';

	onMount(() => {
		void discovery.scan();
	});

	function ago(ts: number | null): string {
		if (!ts) return 'never';
		const s = Math.max(0, Math.round((Date.now() - ts) / 1000));
		if (s < 2) return 'just now';
		if (s < 60) return `${s}s ago`;
		return `${Math.floor(s / 60)}m ${s % 60}s ago`;
	}

	const neighborRows = $derived(discovery.visibleNeighbors);
	const ifaceList = $derived(discovery.networkInterfaces);
	const scanned = $derived(ago(discovery.lastScan));

	let listCopied = $state(false);

	async function copyList() {
		const text = discovery.visibleListText();
		if (!text) return;
		const ok = await copyText(text);
		listCopied = ok;
		setTimeout(() => {
			listCopied = false;
		}, 1400);
	}

	function scopeLabel(scope: string): string {
		if (scope === 'unique-local') return 'ULA';
		if (scope === 'link-local') return 'LL';
		return scope;
	}
</script>

<div class="shell">
	<header class="top">
		<div class="brand">
			<span class="mark" aria-hidden="true"></span>
			<div>
				<div class="title">Lightning Admin</div>
				<div class="sub">Unique Local IPv6 as base58 · hover for the address</div>
			</div>
		</div>
		<div class="actions">
			<div class="scopes" role="tablist" aria-label="Address scope">
				<button
					type="button"
					class="scan ghost"
					class:on={discovery.selectedScope === 'unique-local'}
					onclick={() => discovery.selectScope('unique-local')}
				>
					ULA {discovery.uniqueLocalCount}
				</button>
				<button
					type="button"
					class="scan ghost"
					class:on={discovery.selectedScope === 'link-local'}
					onclick={() => discovery.selectScope('link-local')}
				>
					LL {discovery.linkLocalCount}
				</button>
				<button
					type="button"
					class="scan ghost"
					class:on={discovery.selectedScope === 'all'}
					onclick={() => discovery.selectScope('all')}
				>
					All {discovery.neighbors.length}
				</button>
			</div>
			<button
				type="button"
				class="scan ghost"
				disabled={neighborRows.length === 0}
				onclick={() => copyList()}
			>
				{listCopied ? 'Copied list' : 'Copy list'}
			</button>
			<button type="button" class="scan" disabled={discovery.loading} onclick={() => discovery.scan()}>
				{discovery.loading ? 'Scanning…' : 'Scan'}
			</button>
		</div>
	</header>

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
			class="scan"
			disabled={!networkName.canApply}
			onclick={() => networkName.apply()}
		>
			{networkName.applying ? 'Applying…' : 'Apply'}
		</button>
		{#if networkName.applying}
			<span class="nn-status">Applying across the fleet…</span>
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

	<div class="body">
		<aside class="rail">
			<div class="rail-head">Interfaces</div>
			<button
				type="button"
				class="iface"
				class:active={discovery.selectedIface === 'all'}
				onclick={() => discovery.selectIface('all')}
			>
				<span class="iname">all</span>
				<span class="icount">{discovery.scopedNeighbors.length}</span>
			</button>
			{#each ifaceList as iface (iface.name)}
				<button
					type="button"
					class="iface"
					class:active={discovery.selectedIface === iface.name}
					class:down={!iface.is_up}
					onclick={() => discovery.selectIface(iface.name)}
				>
					<span class="idot" class:up={iface.is_up}></span>
					<span class="iname">{iface.name}</span>
					<span class="icount">{discovery.neighborCount(iface.name)}</span>
				</button>
			{/each}
			{#if ifaceList.length === 0 && !discovery.loading}
				<p class="empty">No non-loopback interfaces.</p>
			{/if}
		</aside>

		<main class="main">
			{#if discovery.error}
				<div class="banner bad">
					Scan failed: {discovery.error}
				</div>
			{/if}
			{#if discovery.probeError}
				<div class="banner warn">
					Neighbor table is shown; ICMPv6 probe: {discovery.probeError}
				</div>
			{/if}

			{#if neighborRows.length === 0 && !discovery.loading && !discovery.error}
				<div class="empty-main">
					{#if discovery.selectedScope === 'unique-local' && discovery.linkLocalCount > 0}
						No Unique Local IPv6 on this view. Switch to LL or All, or join a mesh that hands out ULA.
					{:else}
						No Unique Local or link-local IPv6 yet. Plug into a LAN (or a mesh node) and Scan.
					{/if}
				</div>
			{:else}
				<table>
					<thead>
						<tr>
							<th>Base58</th>
							<th>Scope</th>
							<th>MAC</th>
							<th>Interface</th>
							<th>Kind</th>
							<th>State</th>
							<th>Source</th>
						</tr>
					</thead>
					<tbody>
						{#each neighborRows as n (`${n.ifindex}:${n.address}`)}
							<tr class:local={n.kind === 'local'} class:ula={n.scope === 'unique-local'}>
								<td class="addr">
									<AddrCell neighbor={n} />
								</td>
								<td>
									<span class="scope {n.scope}">{scopeLabel(n.scope)}</span>
								</td>
								<td class="mac">{n.mac ?? '—'}</td>
								<td>{n.iface}</td>
								<td>
									<span class="kind {n.kind}">{n.kind === 'local' ? 'this host' : 'neighbor'}</span>
								</td>
								<td class="state {n.state}">{n.state}</td>
								<td class="src">{n.source}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</main>
	</div>

	<footer class="status">
		<span>{ifaceList.length} iface{ifaceList.length === 1 ? '' : 's'}</span>
		<span class="sep">·</span>
		<span>{discovery.uniqueLocalCount} ULA</span>
		<span class="sep">·</span>
		<span>{discovery.linkLocalCount} link-local</span>
		<span class="sep">·</span>
		<span>{discovery.probed ? 'probed ff02::1' : 'no probe'}</span>
		<span class="sep">·</span>
		<span>last scan {scanned}</span>
	</footer>
</div>

<style>
	.shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background:
			radial-gradient(1200px 400px at 10% -10%, #12352b 0%, transparent 55%),
			var(--ink);
	}

	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.85rem 1rem 0.75rem;
		border-bottom: 1px solid var(--line);
		background: color-mix(in srgb, var(--panel) 86%, black);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.mark {
		width: 10px;
		height: 28px;
		background: linear-gradient(180deg, var(--mint), #0b5c47);
		box-shadow: 0 0 12px #3ee0b088;
	}

	.title {
		font-size: 13px;
		letter-spacing: 0.16em;
		text-transform: uppercase;
		color: var(--mint);
	}

	.sub {
		color: var(--muted);
		font-size: 11px;
		margin-top: 2px;
	}

	.scan {
		border: 1px solid var(--mint-dim);
		background: #0e2a22;
		color: var(--mint);
		padding: 0.4rem 0.9rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		font-size: 11px;
	}

	.scan:hover:not(:disabled) {
		border-color: var(--mint);
		background: #12362c;
	}

	.scan.ghost {
		border-color: var(--line-strong);
		background: transparent;
		color: var(--text);
	}

	.actions {
		display: flex;
		gap: 0.45rem;
		align-items: center;
	}

	.scopes {
		display: flex;
		gap: 2px;
		margin-right: 0.35rem;
	}

	.scan.ghost.on {
		border-color: var(--mint);
		color: var(--mint);
		background: #12362c;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 220px 1fr;
	}

	.rail {
		border-right: 1px solid var(--line);
		background: var(--panel);
		padding: 0.6rem 0.45rem;
		overflow: auto;
	}

	.rail-head {
		font-size: 10px;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--muted);
		padding: 0.25rem 0.55rem 0.5rem;
	}

	.iface {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		width: 100%;
		border: 1px solid transparent;
		background: transparent;
		text-align: left;
		padding: 0.38rem 0.5rem;
		margin-bottom: 2px;
	}

	.iface:hover {
		background: var(--row-hover);
	}

	.iface.active {
		border-color: var(--line-strong);
		background: #16362c;
	}

	.iface.down .iname {
		color: var(--muted);
	}

	.idot {
		width: 7px;
		height: 7px;
		border-radius: 99px;
		background: #4a5c54;
		flex: 0 0 auto;
	}

	.idot.up {
		background: var(--ok);
		box-shadow: 0 0 6px #7dff9a88;
	}

	.iname {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.icount {
		color: var(--muted);
		font-variant-numeric: tabular-nums;
		font-size: 11px;
	}

	.main {
		overflow: auto;
		padding: 0;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}

	th {
		position: sticky;
		top: 0;
		text-align: left;
		font-size: 10px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--muted);
		background: var(--panel-2);
		border-bottom: 1px solid var(--line);
		padding: 0.55rem 0.75rem;
		font-weight: 500;
	}

	td {
		padding: 0.45rem 0.75rem;
		border-bottom: 1px solid var(--line);
		vertical-align: middle;
	}

	tr:hover td {
		background: var(--row-hover);
	}

	tr.local td {
		background: color-mix(in srgb, var(--local-bg) 70%, transparent);
	}

	.addr {
		color: var(--mint);
		font-size: 12.5px;
		overflow: visible;
	}

	.scope {
		display: inline-block;
		padding: 0.08rem 0.4rem;
		border: 1px solid var(--line-strong);
		font-size: 10px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}

	.scope.unique-local {
		color: var(--mint);
		border-color: var(--mint-dim);
	}

	.scope.link-local {
		color: var(--muted);
	}

	.mac,
	.src {
		color: var(--muted);
	}

	.kind {
		display: inline-block;
		padding: 0.08rem 0.4rem;
		border: 1px solid var(--line-strong);
		font-size: 10px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}

	.kind.local {
		color: var(--amber);
		border-color: #6a5420;
	}

	.kind.neighbor {
		color: var(--text);
	}

	.state.reachable,
	.state.local {
		color: var(--ok);
	}

	.state.stale,
	.state.delay,
	.state.probe {
		color: var(--warn);
	}

	.banner {
		margin: 0.75rem 0.75rem 0;
		padding: 0.5rem 0.65rem;
		border: 1px solid var(--line);
		font-size: 12px;
	}

	.banner.bad {
		border-color: #6a2e2c;
		color: var(--bad);
	}

	.banner.warn {
		border-color: #5a4a22;
		color: var(--warn);
	}

	.empty,
	.empty-main {
		color: var(--muted);
		padding: 1.25rem 0.85rem;
		font-size: 12px;
	}

	.status {
		display: flex;
		gap: 0.45rem;
		align-items: center;
		padding: 0.4rem 0.85rem;
		border-top: 1px solid var(--line);
		color: var(--muted);
		font-size: 11px;
		background: var(--panel);
	}

	.sep {
		opacity: 0.5;
	}

	.nn-bar {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.45rem 1rem;
		border-bottom: 1px solid var(--line);
		background: var(--panel);
		flex-wrap: wrap;
	}

	.nn-bar label {
		font-size: 10px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--muted);
		white-space: nowrap;
	}

	.nn-bar input {
		flex: 1;
		min-width: 12rem;
		background: #0e2a22;
		border: 1px solid var(--mint-dim);
		color: var(--text);
		padding: 0.35rem 0.6rem;
		font: inherit;
	}

	.nn-bar input:focus {
		outline: none;
		border-color: var(--mint);
	}

	.nn-status {
		color: var(--muted);
		font-size: 11px;
		min-width: 0;
	}

	.nn-status.bad {
		color: var(--bad);
	}

	.nn-log {
		flex-basis: 100%;
		color: var(--muted);
		font-size: 11px;
	}

	.nn-log summary {
		cursor: pointer;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		font-size: 10px;
	}

	.nn-log pre {
		margin: 0.35rem 0 0;
		max-height: 8rem;
		overflow: auto;
		padding: 0.45rem 0.55rem;
		border: 1px solid var(--line);
		background: var(--panel-2);
		color: var(--text);
		font: inherit;
		font-size: 11px;
		white-space: pre-wrap;
	}
</style>
