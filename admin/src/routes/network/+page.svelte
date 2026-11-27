<script lang="ts">
	import { onMount } from 'svelte';
	import { openCanvas } from '$lib/canvas/open';
	import { copyText } from '$lib/clipboard';
	import FilterMenu from '$lib/discovery/FilterMenu.svelte';
	import IfaceMenu from '$lib/discovery/IfaceMenu.svelte';
	import { classifyIface, ifaceTitle, KIND_ORDER, kindLabel, type IfaceKind } from '$lib/discovery/groups';
	import { lightningDetail, type LightningDetail } from '$lib/discovery/lightning-detail';
	import { formatHops, gatewayOnInterface, hopDistances, macFromIpv6 } from '$lib/discovery/mesh-layers';
	import Radar from '$lib/discovery/Radar.svelte';
	import {
		DEFAULT_FILTERS,
		isLightning,
		resourceMatches,
		stateBucket,
		type ListFilters
	} from '$lib/discovery/list-filter';
	import { formatVia, groupResources, resourceAddress, resourceLabel, type Resource } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { fetchLanJson } from '$lib/node/fetch';
	import { asNodeDocument, asRadioDocument } from '$lib/node/documents';
	import { lanTarget } from '$lib/node/lan';

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

	const ifaceList = $derived(discovery.networkInterfaces);
	const scanned = $derived(ago(discovery.lastScan));

	const PRIMARY = new Set<IfaceKind>(['ethernet', 'wifi', 'vpn']);
	let showAll = $state(false);
	let selectedIfaces = $state<string[]>([]);
	let filters = $state<ListFilters>({ ...DEFAULT_FILTERS });
	let lightningRows = $state<Record<string, LightningDetail>>({});
	let meshOnly = $state<{ mac: string; hops: number }[]>([]);
	let meshDepth = $state<number | null>(null);
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
				return resourceAddress(resource);
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

	const observed = $derived(discovery.neighbors);

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

	const viewRows = $derived.by(() =>
		observed.filter((n) => {
			if (selectedIfaces.length > 0 && !selectedIfaces.includes(n.iface)) return false;
			if (showAll) return true;
			const kind = classifyIface(n.iface);
			return kind !== 'bridge' && kind !== 'virtual';
		})
	);

	const resources = $derived(groupResources(viewRows));
	const filterCounts = $derived.by(() => {
		const counts = { live: 0, stale: 0, failed: 0, lightning: 0, gateway: 0 };
		for (const resource of resources) {
			counts[stateBucket(resource.state)] += 1;
			if (isLightning(resource)) counts.lightning += 1;
			if (resource.router) counts.gateway += 1;
		}
		return counts;
	});
	const listedResources = $derived(
		resources.filter(
			(resource) =>
				resourceMatches(resource, filters) &&
				(!filters.lightning || resource.rows.some((row) => row.kind !== 'directory'))
		)
	);

	let expanded = $state<Record<string, boolean>>({});

	const lightningKey = $derived(
		filters.lightning
			? listedResources.map((resource) => `${resource.id}:${lanTarget(resource.rows)?.address ?? ''}`).join('|')
			: ''
	);

	$effect(() => {
		const key = lightningKey;
		if (!key) {
			lightningRows = {};
			meshOnly = [];
			meshDepth = null;
			return;
		}
		const targets = listedResources.filter((resource) => isLightning(resource));
		const localAddrs = discovery.interfaces.flatMap((iface) => iface.ipv4 ?? []);
		let cancelled = false;
		void (async () => {
			const fetched = await Promise.all(
				targets.map(async (resource) => {
					const target = lanTarget(resource.rows);
					let node: unknown = null;
					let radio: unknown = null;
					let health: unknown = null;
					let healthError: string | null = null;
					if (target) {
						const [nodeResult, radioResult, healthResult] = await Promise.allSettled([
							fetchLanJson(target.address, '/api/node', target.scopeId),
							fetchLanJson(target.address, '/api/radio', target.scopeId),
							fetchLanJson(target.address, '/api/health', target.scopeId)
						]);
						if (nodeResult.status === 'fulfilled') node = nodeResult.value;
						if (radioResult.status === 'fulfilled') radio = radioResult.value;
						if (healthResult.status === 'fulfilled') health = healthResult.value;
						else {
							healthError =
								healthResult.reason instanceof Error
									? healthResult.reason.message
									: String(healthResult.reason);
						}
					} else {
						healthError = 'No LAN address on the last scan.';
					}
					const parsedNode = (() => {
						try {
							return node ? asNodeDocument(node) : null;
						} catch {
							return null;
						}
					})();
					const parsedRadio = (() => {
						try {
							return radio ? asRadioDocument(radio) : null;
						} catch {
							return null;
						}
					})();
					const mac =
						parsedRadio?.mesh_mac?.trim().toLowerCase() ||
						(parsedNode?.link_local_mesh ? macFromIpv6(parsedNode.link_local_mesh) : null);
					return {
						resource,
						mac,
						radio: parsedRadio,
						detail: lightningDetail({
							iface: resource.primary.iface,
							state: resourceState(resource),
							node,
							radio,
							health,
							healthError
						})
					};
				})
			);
			if (cancelled) return;
			const vantageId = gatewayOnInterface(
				localAddrs,
				fetched.map((item) => ({
					id: item.resource.id,
					addresses: item.resource.rows.map((row) => row.address)
				}))
			);
			const vantage = fetched.find((item) => item.resource.id === vantageId);
			const distances = vantage?.mac
				? hopDistances(
						vantage.mac,
						(vantage.radio?.mpaths ?? []).map((path) => ({ dst: path.dst, nextHop: path.next_hop })),
						(vantage.radio?.stations ?? []).map((station) => station.mac)
					)
				: new Map<string, number>();
			const known = new Set(fetched.map((item) => item.mac).filter((mac): mac is string => Boolean(mac)));
			const next: Record<string, LightningDetail> = {};
			let depth: number | null = null;
			for (const item of fetched) {
				let hops = item.mac ? (distances.get(item.mac) ?? null) : null;
				if (item.resource.id === vantageId) hops = 0;
				if (hops !== null) depth = depth === null ? hops : Math.max(depth, hops);
				next[item.resource.id] = { ...item.detail, hops: formatHops(hops) };
			}
			const extras: { mac: string; hops: number }[] = [];
			for (const [mac, hops] of distances) {
				if (known.has(mac)) continue;
				extras.push({ mac, hops });
				depth = depth === null ? hops : Math.max(depth, hops);
			}
			extras.sort((a, b) => a.hops - b.hops || a.mac.localeCompare(b.mac));
			lightningRows = next;
			meshOnly = extras;
			meshDepth = depth;
		})();
		return () => {
			cancelled = true;
		};
	});

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

	function hopRank(label: string | undefined): number {
		if (!label || label === '…' || label === '—') return 99;
		if (label === 'here') return 0;
		const hops = Number.parseInt(label, 10);
		return Number.isFinite(hops) ? hops : 99;
	}

	const shownResources = $derived.by(() => {
		if (!filters.lightning) return sortedResources;
		return [...sortedResources].sort(
			(a, b) =>
				hopRank(lightningRows[a.id]?.hops) - hopRank(lightningRows[b.id]?.hops) ||
				(a.meshName ?? a.primary.address).localeCompare(b.meshName ?? b.primary.address)
		);
	});

	function toggleShowAll() {
		showAll = !showAll;
		if (!showAll) {
			selectedIfaces = selectedIfaces.filter((name) => PRIMARY.has(classifyIface(name)));
		}
	}

	let listCopied = $state(false);
	let canvasError = $state('');

	async function addCanvas() {
		canvasError = '';
		try {
			await openCanvas();
		} catch (error) {
			canvasError = String(error);
		}
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
		<div class="actions">
			<FilterMenu {filters} counts={filterCounts} onchange={(next) => (filters = next)} />
			<span class="sep" aria-hidden="true">|</span>
			<IfaceMenu
				{groups}
				selected={selectedIfaces}
				{showAll}
				{hiddenCount}
				onSelect={(next) => (selectedIfaces = next)}
				onShowAll={toggleShowAll}
			/>
			<button
				type="button"
				class="scan ghost"
				disabled={sortedResources.length === 0}
				onclick={() => copyList()}
			>
				{listCopied ? 'Copied list' : 'Copy list'}
			</button>
			<Radar scanning={discovery.loading} onclick={() => discovery.scan()} />
			<button type="button" class="scan ghost" onclick={() => void addCanvas()}>Canvas</button>
			{#if canvasError}
				<span class="nn-status bad" role="alert">{canvasError}</span>
			{/if}
		</div>
	</header>

	<div class="body">
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

			{#if resources.length > 0 && listedResources.length === 0 && !discovery.loading && !discovery.error}
				<div class="empty-main">
					{#if filters.lightning}
						No Lightning router is on this mesh.
					{:else}
						Nothing matches these filters.
					{/if}
					{#if filterCounts.stale > 0 && !filters.stale}
						{filterCounts.stale} stale {filterCounts.stale === 1 ? 'entry is' : 'entries are'} hidden.
					{/if}
				</div>
			{:else if listedResources.length === 0 && !discovery.loading && !discovery.error}
				<div class="empty-main">
					{#if filters.lightning}
						No Lightning router is on this mesh.
					{:else}
						Nothing answered yet. The scan reads IPv4 ARP and IPv6 neighbors, marks routers, and asks Avahi for hostnames.
					{/if}
				</div>
			{:else if shownResources.length > 0 || meshOnly.length > 0}
				<table class:lightning={filters.lightning}>
					<thead>
						<tr>
							<th></th>
							{#if filters.lightning}
								<th>Name</th>
								<th title="Mesh hops from the router this computer is joined to">Hops</th>
								<th title="Mesh interface and channel, or the interface that heard this node">Link</th>
								<th title="Strongest peer the node's radio reports">Strength</th>
								<th title="Software version, when the node publishes one">Version</th>
							{:else}
								{#each [{ key: 'address', label: 'Name' }, { key: 'where', label: 'Address' }, { key: 'mac', label: 'Via' }, { key: 'state', label: 'State' }] as col (col.key)}
									<th aria-sort={sortKey === col.key ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'}>
										<button type="button" class="sort" onclick={() => toggleSort(col.key as SortKey)}>
											{col.label}{sortKey === col.key ? (sortDir === 'asc' ? ' ↑' : ' ↓') : ''}
										</button>
									</th>
								{/each}
							{/if}
						</tr>
					</thead>
					<tbody>
						{#each shownResources as resource (resource.id)}
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
									<strong>{filters.lightning ? resource.meshName || resourceAddress(resource) : resourceLabel(resource)}</strong>
									{#if isLightning(resource) && !filters.lightning}
										<span class="chip">Lightning</span>
									{:else if resource.router && !filters.lightning}
										<span class="chip quiet">gateway</span>
									{/if}
									{#if filters.lightning && resource.meshName}
										<div class="muted">{resourceAddress(resource)}</div>
									{:else}
										{#if resource.meshSubnet}
											<div class="muted">{resource.meshSubnet}</div>
										{/if}
										{#if resource.meshBackhaul}
											<div class="muted">Transport pin: {resource.meshBackhaul}</div>
										{/if}
									{/if}
									{#if resource.rows.length > 1}
										<span class="more">+{resource.rows.length - 1}</span>
									{/if}
								</td>
								{#if filters.lightning}
									{@const detail = lightningRows[resource.id]}
									<td>{detail?.hops ?? '…'}</td>
									<td class="link">{detail?.link ?? '…'}</td>
									<td>{detail?.strength ?? '…'}</td>
									<td>{detail?.version ?? '…'}</td>
								{:else}
									<td>
										<button type="button" class="plain-addr" onclick={() => copyText(resourceAddress(resource))}>
											{resourceAddress(resource)}
										</button>
										{#if resource.houseUla}<span class="muted"> /128 reference</span>{/if}
									</td>
									<td class="via" title={resource.via.join(', ')}>{resource.local ? '—' : formatVia(resource.via) || ifaceTitle(resource.primary.iface)}</td>
									<td class="state {resource.state}">{resourceState(resource)}</td>
								{/if}
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
										<td class="state {n.state}" colspan={filters.lightning ? 2 : 1}>{stateText(n.kind, n.state)}</td>
									</tr>
								{/each}
							{/if}
						{/each}
						{#if filters.lightning}
							{#each meshOnly as peer (peer.mac)}
								<tr>
									<td></td>
									<td class="addr"><strong>{peer.mac}</strong></td>
									<td>{formatHops(peer.hops)}</td>
									<td class="link">—</td>
									<td>—</td>
									<td>—</td>
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			{/if}
		</main>
	</div>

	<footer class="status">
		<span>{ifaceList.length} iface{ifaceList.length === 1 ? '' : 's'}</span>
		<span class="sep">·</span>
		<span>{discovery.probed ? 'probed ff02::1' : 'no probe'}</span>
		<span class="sep">·</span>
		<span>last scan {scanned}</span>
		{#if filters.lightning && meshDepth !== null}
			<span class="sep">·</span>
			<span>{listedResources.length + meshOnly.length} router{listedResources.length + meshOnly.length === 1 ? '' : 's'}</span>
			<span class="sep">·</span>
			<span>{meshDepth === 0 ? 'on this LAN' : `${meshDepth} ${meshDepth === 1 ? 'hop' : 'hops'} deep`}</span>
		{/if}
	</footer>
</div>

<style>
	.shell {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		min-width: 0;
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
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
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

	.body {
		flex: 1;
		min-height: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
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

	.main {
		overflow: auto;
		min-height: 0;
		min-width: 0;
		scrollbar-width: thin;
		scrollbar-color: var(--border-strong) transparent;
	}

	.main::-webkit-scrollbar {
		height: 10px;
	}

	.main::-webkit-scrollbar-thumb {
		background: var(--border-strong);
		border-radius: var(--radius);
	}

	table {
		width: 100%;
		min-width: 28rem;
		table-layout: fixed;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}

	th:nth-child(1),
	td:nth-child(1) {
		width: 2rem;
	}

	th:nth-child(4),
	td:nth-child(4) {
		width: 8.5rem;
	}

	th:nth-child(5),
	td:nth-child(5) {
		width: 7.5rem;
	}

	table.lightning {
		min-width: 40rem;
	}

	table.lightning th:nth-child(3),
	table.lightning td:nth-child(3) {
		width: 9rem;
	}

	table.lightning th:nth-child(4),
	table.lightning td:nth-child(4),
	table.lightning th:nth-child(5),
	table.lightning td:nth-child(5) {
		width: 6.5rem;
	}

	table.lightning th:nth-child(6),
	table.lightning td:nth-child(6) {
		width: 9.5rem;
	}

	th:nth-child(2),
	td:nth-child(2),
	th:nth-child(3),
	td:nth-child(3) {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	td.via,
	td.mac,
	td.state {
		white-space: nowrap;
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

	.nn-status.bad {
		color: var(--bad);
		font-size: 13px;
	}

</style>
