<script lang="ts">
	import { onMount } from 'svelte';
	import { DropdownMenu } from 'bits-ui';
	import { openCanvas } from '$lib/canvas/open';
	import { copyText } from '$lib/clipboard';
	import AddrCell from '$lib/discovery/AddrCell.svelte';
	import { classifyIface, ifaceTitle, KIND_ORDER, kindLabel, type IfaceKind } from '$lib/discovery/groups';
	import { groupResources, resourceLabel, type Resource } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import GuildPanel from '$lib/guild/mock/GuildPanel.svelte';
	import { NETWORK_NAME_LABEL, networkName } from '$lib/network-name/store.svelte';
	import {
		applyAppearance,
		readAppearance,
		THEME_NAMES,
		VARIANT_CHOICES,
		type ThemeName,
		type VariantChoice
	} from '$lib/theme';

	onMount(() => {
		const saved = readAppearance();
		themeName = saved.name;
		variantChoice = saved.variant;
		void discovery.scan();
	});

	function ago(ts: number | null): string {
		if (!ts) return 'never';
		const s = Math.max(0, Math.round((Date.now() - ts) / 1000));
		if (s < 2) return 'just now';
		if (s < 60) return `${s}s ago`;
		return `${Math.floor(s / 60)}m ${s % 60}s ago`;
	}

	const ifaceList = $derived(discovery.networkInterfaces);
	const scanned = $derived(ago(discovery.lastScan));

	type View = 'resources' | IfaceKind;
	const PRIMARY = new Set<IfaceKind>(['ethernet', 'wifi', 'vpn']);
	let view = $state<View>('resources');
	let showAll = $state(false);
	let lightningOnly = $state(false);
	type SortKey = 'address' | 'where' | 'mac' | 'state' | 'scope' | 'iroh';
	let sortKey = $state<SortKey | null>(null);
	let sortDir = $state<'asc' | 'desc'>('asc');

	function toggleSort(key: SortKey) {
		if (sortKey === key) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	function stateText(kind: string, state: string): string {
		return kind === 'local' ? 'this computer' : state;
	}

	function resourceState(resource: Resource): string {
		if (resource.rows.every((row) => row.kind === 'local')) return 'this computer';
		return resource.state || 'unknown';
	}

	function sortValue(resource: Resource, key: SortKey): string {
		switch (key) {
			case 'address':
				return resourceLabel(resource);
			case 'where':
				return resource.primary.address;
			case 'mac':
				return resource.via.join(',');
			case 'state':
				return resourceState(resource);
			case 'scope':
				return resource.primary.scope;
			case 'iroh':
				return resource.iroh ?? '';
		}
	}

	const observed = $derived(
		discovery.neighbors.filter((n) => showAll || n.state !== 'failed')
	);

	const groups = $derived(
		KIND_ORDER.map((kind) => ({
			kind,
			label: kindLabel(kind),
			ifaces: ifaceList.filter((i) => classifyIface(i.name) === kind)
		})).filter((g) => g.ifaces.length > 0 && (showAll || PRIMARY.has(g.kind)))
	);

	const hiddenCount = $derived(
		ifaceList.filter((i) => !PRIMARY.has(classifyIface(i.name))).length
	);

	const viewRows = $derived.by(() => {
		const pool = observed.filter((n) => {
			if (showAll || view !== 'resources') return true;
			const kind = classifyIface(n.iface);
			return kind !== 'bridge' && kind !== 'virtual';
		});
		if (view === 'resources') return pool;
		const names = new Set(
			ifaceList.filter((i) => classifyIface(i.name) === view).map((i) => i.name)
		);
		return pool.filter((n) => names.has(n.iface));
	});

	const viewIfaces = $derived(
		view === 'resources' ? [] : ifaceList.filter((i) => classifyIface(i.name) === view)
	);

	const resources = $derived(groupResources(viewRows));
	const lightningResources = $derived(resources.filter((resource) => resource.meshName));
	const listedResources = $derived(lightningOnly ? lightningResources : resources);
	let expanded = $state<Record<string, boolean>>({});

	const sortedResources = $derived.by(() => {
		if (!sortKey) return listedResources;
		const dir = sortDir === 'asc' ? 1 : -1;
		const key = sortKey;
		return [...listedResources].sort(
			(a, b) =>
				dir * sortValue(a, key).localeCompare(sortValue(b, key)) ||
				a.primary.address.localeCompare(b.primary.address)
		);
	});

	function toggleShowAll() {
		showAll = !showAll;
		if (!showAll && view !== 'resources' && !PRIMARY.has(view)) view = 'resources';
	}

	let listCopied = $state(false);
	let themeName = $state<ThemeName>('plain');
	let variantChoice = $state<VariantChoice>('system');

	function setAppearance(next: { name?: ThemeName; variant?: VariantChoice }) {
		if (next.name) themeName = next.name;
		if (next.variant) variantChoice = next.variant;
		applyAppearance({ name: themeName, variant: variantChoice });
	}

	async function copyList() {
		const text = sortedResources
			.flatMap((resource) => resource.rows.map((n) => `${n.address}\t${n.iface}\t${n.mac ?? ''}`))
			.join('\n');
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
				<a class="become" href="/become">Become the admin</a>
				<div class="sub">Machines on the links this computer can hear, with the name and address each one announced.</div>
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
				disabled={sortedResources.length === 0}
				onclick={() => copyList()}
			>
				{listCopied ? 'Copied list' : 'Copy list'}
			</button>
			<button type="button" class="scan primary" disabled={discovery.loading} onclick={() => discovery.scan()}>
				{discovery.loading ? 'Scanning…' : 'Scan'}
			</button>
			<button type="button" class="scan ghost" onclick={() => void openCanvas()}>Canvas</button>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<button type="button" class="scan ghost" {...props}>Appearance</button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Portal>
					<DropdownMenu.Content class="lm-menu" align="end" sideOffset={6}>
						<DropdownMenu.Group>
							<DropdownMenu.GroupHeading class="lm-menu-label">Theme</DropdownMenu.GroupHeading>
							<DropdownMenu.RadioGroup
								value={themeName}
								onValueChange={(value) => setAppearance({ name: value as ThemeName })}
							>
								{#each THEME_NAMES as theme (theme.id)}
									<DropdownMenu.RadioItem class="lm-menu-item" value={theme.id} closeOnSelect={false}>
										{#snippet children({ checked })}
											<span class="lm-menu-mark" aria-hidden="true">{checked ? '●' : ''}</span>
											{theme.label}
										{/snippet}
									</DropdownMenu.RadioItem>
								{/each}
							</DropdownMenu.RadioGroup>
						</DropdownMenu.Group>
						<DropdownMenu.Separator class="lm-menu-sep" />
						<DropdownMenu.Group>
							<DropdownMenu.GroupHeading class="lm-menu-label">Variant</DropdownMenu.GroupHeading>
							<DropdownMenu.RadioGroup
								value={variantChoice}
								onValueChange={(value) => setAppearance({ variant: value as VariantChoice })}
							>
								{#each VARIANT_CHOICES as variant (variant.id)}
									<DropdownMenu.RadioItem class="lm-menu-item" value={variant.id} closeOnSelect={false}>
										{#snippet children({ checked })}
											<span class="lm-menu-mark" aria-hidden="true">{checked ? '●' : ''}</span>
											{variant.label}
										{/snippet}
									</DropdownMenu.RadioItem>
								{/each}
							</DropdownMenu.RadioGroup>
						</DropdownMenu.Group>
					</DropdownMenu.Content>
				</DropdownMenu.Portal>
			</DropdownMenu.Root>
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
			class="scan primary"
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

	<GuildPanel />

	<div class="body">
		<aside class="rail">
			<div class="rail-head">Look at</div>
			<button type="button" class="iface" class:active={view === 'resources'} onclick={() => (view = 'resources')}>
				<span class="iname">Resources</span>
				<span class="icount">{view === 'resources' ? resources.length : groupResources(observed.filter((n) => showAll || (classifyIface(n.iface) !== 'bridge' && classifyIface(n.iface) !== 'virtual'))).length}</span>
			</button>
			<button
				type="button"
				class="iface"
				class:active={lightningOnly}
				onclick={() => (lightningOnly = !lightningOnly)}
			>
				<span class="iname">Lightning nodes</span>
				<span class="icount">{lightningResources.length}</span>
			</button>
			<div class="rail-head">Interfaces</div>
			{#each groups as group (group.kind)}
				<button
					type="button"
					class="iface"
					class:active={view === group.kind}
					onclick={() => (view = group.kind)}
				>
					<span class="idot" class:up={group.ifaces.some((i) => i.is_up)}></span>
					<span class="iname">{group.label}</span>
					<span class="icount">{group.ifaces.length}</span>
				</button>
			{/each}
			{#if hiddenCount > 0 || showAll}
				<button type="button" class="show-all" onclick={toggleShowAll}>
					{showAll ? 'Hide extra' : `Show all · ${hiddenCount}`}
				</button>
			{/if}
			{#if showAll && view !== 'resources'}
				{#each viewIfaces as iface (iface.name)}
					<button
						type="button"
						class="iface sub"
						class:down={!iface.is_up}
						onclick={() => discovery.selectIface(iface.name)}
					>
						<span class="iname" title={iface.name}>{iface.name}</span>
					</button>
				{/each}
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

			{#if view !== 'resources' && viewIfaces.length > 0}
				<ul class="cards">
					{#each viewIfaces as iface (iface.name)}
						<li>
							<strong>{ifaceTitle(iface.name)}</strong>
							<span class="muted">{iface.is_up ? 'up' : 'down'}</span>
							{#if showAll}
								<code>{iface.name}</code>
							{/if}
							{#if iface.unique_local.length}
								<span class="addr-line">{iface.unique_local[0]}</span>
							{:else if showAll && iface.link_local.length}
								<span class="addr-line">{iface.link_local[0]}</span>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			{#if listedResources.length === 0 && !discovery.loading && !discovery.error}
				<div class="empty-main">
					{#if lightningOnly}
						No Lightning node answered on this view. A node is recognized when one of its addresses answers /api/node.
					{:else if view === 'resources'}
						Nothing answered yet. The scan reads IPv4 ARP and IPv6 neighbors, marks routers, and asks Avahi for hostnames. Failed ARP entries stay hidden until Show all.
					{:else}
						No addresses on {kindLabel(view)}.
					{/if}
				</div>
			{:else if sortedResources.length > 0}
				<table>
					<thead>
						<tr>
							<th></th>
							{#each [{ key: 'address', label: 'Name' }, { key: 'where', label: 'Address' }, { key: 'mac', label: 'Via' }, { key: 'state', label: 'State' }] as col (col.key)}
								<th aria-sort={sortKey === col.key ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'}>
									<button type="button" class="sort" onclick={() => toggleSort(col.key as SortKey)}>
										{col.label}{sortKey === col.key ? (sortDir === 'asc' ? ' ↑' : ' ↓') : ''}
									</button>
								</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each sortedResources as resource (resource.id)}
							<tr class:local={resource.primary.kind === 'local'}>
								<td class="twist">
									{#if resource.rows.length > 1}
										<button
											type="button"
											class="twist-btn"
											aria-expanded={expanded[resource.id] ?? false}
											aria-label={expanded[resource.id] ? 'Hide addresses' : 'Show addresses'}
											onclick={() => (expanded[resource.id] = !expanded[resource.id])}
										>
											{expanded[resource.id] ? '▾' : '▸'}
										</button>
									{/if}
								</td>
								<td class="addr">
									<strong>{resourceLabel(resource)}</strong>
									{#if resource.meshName}
										<span class="chip">Lightning</span>
									{:else if resource.router}
										<span class="chip quiet">gateway</span>
									{/if}
									{#if resource.meshSubnet}
										<div class="muted">{resource.meshSubnet} · {resource.meshBackhaul}</div>
									{/if}
									{#if resource.rows.length > 1}
										<span class="more">+{resource.rows.length - 1}</span>
									{/if}
								</td>
								<td>
									<button type="button" class="plain-addr" onclick={() => copyText(resource.primary.address)}>
										{resource.primary.address}
									</button>
								</td>
								<td title={resource.via.join(', ')}>{resource.local ? '—' : resource.via.join(', ') || ifaceTitle(resource.primary.iface)}</td>
								<td class="state {resource.state}">{resourceState(resource)}</td>
							</tr>
							{#if resource.rows.length > 1 && expanded[resource.id]}
								{#each resource.rows as n (`${n.ifindex}:${n.address}`)}
									<tr class="detail" class:local={n.kind === 'local'}>
										<td></td>
										<td class="addr">
											<span class="scope {n.scope}">{n.router ? 'gateway' : scopeLabel(n.scope)}</span>
											<span class="muted"> {n.iface}</span>
										</td>
										<td>
											<button type="button" class="plain-addr" onclick={() => copyText(n.scoped || n.address)}>
												{n.address}
											</button>
										</td>
										<td class="mac">{n.mac ?? '—'}</td>
										<td class="state {n.state}">{stateText(n.kind, n.state)}</td>
									</tr>
								{/each}
							{/if}
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
		background: var(--canvas);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.mark {
		display: var(--show-mark);
		width: 10px;
		height: 28px;
		background: linear-gradient(180deg, var(--accent), #0b5c47);
		box-shadow: 0 0 12px color-mix(in srgb, var(--accent) 50%, transparent);
	}

	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem 1rem;
		flex-wrap: wrap;
		padding: 0.7rem 1rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}

	.become {
		display: inline-block;
		margin-top: 0.15rem;
		color: var(--accent);
		font-size: 0.8rem;
		text-decoration: none;
	}

	.title {
		font-size: 15px;
		font-weight: 600;
		letter-spacing: var(--title-track);
		text-transform: var(--case);
		color: var(--title-color);
	}

	.sub {
		color: var(--muted);
		font-size: 13px;
		margin-top: 1px;
	}

	.scan {
		min-height: 36px;
		border: 1px solid var(--border-strong);
		background: var(--surface);
		color: var(--text);
		padding: 0.3rem 0.75rem;
		border-radius: var(--radius);
		font-size: 13px;
	}

	.scan:hover:not(:disabled) {
		background: var(--row-hover);
	}

	.scan.primary {
		background: var(--primary-bg);
		border-color: var(--primary-border);
		color: var(--primary-fg);
	}

	.scan.primary:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	.scan.ghost {
		background: transparent;
	}

	.actions {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		flex-wrap: wrap;
	}

	.scopes {
		display: flex;
		gap: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.scopes .scan {
		border: 0;
		border-radius: 0;
		border-right: 1px solid var(--border);
	}

	.scopes .scan:last-child {
		border-right: 0;
	}

	.scan.ghost.on {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.nn-bar input {
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

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	@media (min-width: 720px) {
		.body {
			display: grid;
			grid-template-columns: 200px 1fr;
		}

		.rail {
			border-bottom: 0;
			border-right: 1px solid var(--border);
			max-height: none;
		}
	}

	.rail {
		border-bottom: 1px solid var(--border);
		background: var(--surface);
		padding: 0.5rem 0.4rem;
		overflow: auto;
	}

	@media (max-width: 719px) {
		.rail {
			max-height: 16rem;
		}
	}

	.rail-head {
		font-size: 12px;
		font-weight: 600;
		color: var(--muted);
		padding: 0.35rem 0.55rem 0.4rem;
		letter-spacing: var(--track);
		text-transform: var(--case);
	}

	.iface {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		min-height: 36px;
		border: 0;
		border-radius: var(--radius);
		background: transparent;
		text-align: left;
		padding: 0.3rem 0.5rem;
	}

	.iface:hover {
		background: var(--row-hover);
	}

	.iface.active {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.iface.sub {
		padding-left: 1.1rem;
		min-height: 28px;
	}

	.show-all {
		display: block;
		width: 100%;
		margin-top: 0.45rem;
		padding: 0.35rem 0.5rem;
		border: 0;
		background: transparent;
		color: var(--muted);
		font-size: 12px;
		text-align: left;
		letter-spacing: 0;
		text-transform: none;
	}

	.show-all:hover {
		color: var(--text);
	}

	.sort {
		border: 0;
		background: transparent;
		padding: 0;
		color: inherit;
		font: inherit;
		letter-spacing: inherit;
		text-transform: inherit;
	}

	.cards {
		list-style: none;
		margin: 0;
		padding: 0.75rem;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.cards li {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 10rem;
		padding: 0.55rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
	}

	.muted,
	.addr-line {
		color: var(--muted);
		font-size: 12px;
	}

	.plain-addr {
		border: 0;
		background: transparent;
		padding: 0;
		color: var(--accent);
		font-family: var(--mono);
		font-size: 13px;
		text-align: left;
		letter-spacing: 0;
		text-transform: none;
	}

	.twist {
		width: 1.6rem;
		padding-right: 0;
	}

	.twist-btn {
		border: 0;
		background: transparent;
		padding: 0 0.2rem;
		color: var(--muted);
		letter-spacing: 0;
		text-transform: none;
	}

	.more {
		margin-left: 0.4rem;
		color: var(--muted);
		font-size: 12px;
		letter-spacing: 0;
		text-transform: none;
	}

	.chip {
		display: inline-block;
		margin-left: 0.4rem;
		padding: 0.05rem 0.4rem;
		border-radius: 999px;
		background: var(--accent-soft);
		color: var(--accent);
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0;
		text-transform: none;
		vertical-align: 1px;
	}

	.chip.quiet {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--muted);
		font-weight: 500;
	}

	tr.detail td {
		background: var(--surface-2);
		font-size: 13px;
	}

	.iface.down .iname {
		color: var(--muted);
	}

	.idot {
		width: 8px;
		height: 8px;
		border-radius: 99px;
		background: var(--border-strong);
		flex: 0 0 auto;
	}

	.idot.up {
		background: var(--ok);
		box-shadow: var(--up-glow);
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
		font-size: 12px;
	}

	.main {
		overflow: auto;
		min-height: 0;
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
		font-size: 12px;
		font-weight: 600;
		color: var(--muted);
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
		padding: 0.5rem 0.75rem;
		letter-spacing: var(--track);
		text-transform: var(--case);
	}

	td {
		padding: 0.45rem 0.75rem;
		border-bottom: 1px solid var(--border);
		vertical-align: middle;
	}

	tr:hover td {
		background: var(--row-hover);
	}

	tr.local td {
		background: var(--local-bg);
	}

	.addr {
		overflow: visible;
	}

	.scope,
	.kind {
		display: inline-block;
		padding: 0.05rem 0.4rem;
		border-radius: 999px;
		background: var(--surface-2);
		border: 1px solid var(--border);
		font-size: 12px;
	}

	.scope.unique-local {
		color: var(--accent);
		background: var(--accent-soft);
		border-color: transparent;
	}

	.scope.link-local,
	.mac,
	.src {
		color: var(--muted);
		font-family: var(--mono);
	}

	.kind.local {
		color: var(--amber);
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
		margin: 0.75rem;
		padding: 0.55rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		font-size: 13px;
	}

	.banner.bad {
		color: var(--bad);
	}

	.banner.warn {
		color: var(--warn);
	}

	.empty,
	.empty-main {
		color: var(--muted);
		padding: 1.25rem 1rem;
		font-size: 14px;
		max-width: 42rem;
	}

	.status {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		flex-wrap: wrap;
		padding: 0.4rem 1rem;
		border-top: 1px solid var(--border);
		color: var(--muted);
		font-size: 12px;
		background: var(--surface);
	}

	.sep {
		opacity: 0.45;
	}

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
