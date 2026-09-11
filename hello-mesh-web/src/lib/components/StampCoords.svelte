<!--
	Phone stamp UI: pick a nearby node (associated default) and enter GPS.
	Confirm POSTs a signed claim to this node's hello; cancel writes nothing.
	Bead mjolnir-mesh-6hn.5 / add-compass-node-mark.
-->
<script lang="ts">
	import { identityStore } from '$lib/identity/store.svelte';
	import { shouldPostStamp, stampTargets, submitCoordinateStamp } from '$lib/directory/stamp';
	import type { Directory } from '$lib/directory/api';
	import type { TopoGraph } from '$lib/topology/graph';
	import { Button } from '$lib/components/ui/button/index.js';
	import { LocateFixed, MapPinned } from '@lucide/svelte';

	let {
		directory,
		graph,
		mock = false
	}: {
		directory: Directory | undefined;
		graph?: TopoGraph;
		mock?: boolean;
	} = $props();

	const inputClass =
		'h-8 w-full rounded-md border border-input bg-background px-2.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring';

	const targets = $derived(stampTargets(directory, graph));
	let selectedNodeId = $state('');
	let latInput = $state('');
	let lonInput = $state('');
	let altInput = $state('');
	let gpsError = $state('');
	let submitError = $state('');
	let status = $state('');
	let busy = $state(false);
	let gpsBusy = $state(false);

	$effect(() => {
		if (targets.length === 0) {
			selectedNodeId = '';
			return;
		}
		if (!targets.some((t) => t.nodeId === selectedNodeId)) {
			selectedNodeId = targets[0].nodeId;
		}
	});

	const selected = $derived(targets.find((t) => t.nodeId === selectedNodeId));

	function parseCoord(s: string): number | null {
		const t = s.trim();
		if (!t) return null;
		const n = Number(t);
		return Number.isFinite(n) ? n : null;
	}

	const lat = $derived(parseCoord(latInput));
	const lon = $derived(parseCoord(lonInput));

	const canConfirm = $derived(
		shouldPostStamp({
			intent: 'confirm',
			targets,
			selectedNodeId,
			lat,
			lon
		}) &&
			(mock || !!identityStore.identity) &&
			!busy
	);

	function formatCoord(nLat?: number, nLon?: number): string {
		if (nLat === undefined || nLon === undefined) return '';
		return `${nLat.toFixed(5)}, ${nLon.toFixed(5)}`;
	}

	async function useGps() {
		gpsError = '';
		if (typeof navigator === 'undefined' || !navigator.geolocation) {
			gpsError = 'No geolocation on this device — type lat/lon.';
			return;
		}
		gpsBusy = true;
		try {
			const pos = await new Promise<GeolocationPosition>((resolve, reject) => {
				navigator.geolocation.getCurrentPosition(resolve, reject, {
					enableHighAccuracy: true,
					timeout: 12000,
					maximumAge: 5000
				});
			});
			latInput = String(pos.coords.latitude);
			lonInput = String(pos.coords.longitude);
			if (pos.coords.altitude != null && Number.isFinite(pos.coords.altitude)) {
				altInput = String(pos.coords.altitude);
			}
		} catch {
			gpsError = 'GPS unavailable — type lat/lon.';
		} finally {
			gpsBusy = false;
		}
	}

	function cancel() {
		latInput = '';
		lonInput = '';
		altInput = '';
		submitError = '';
		status = '';
		gpsError = '';
	}

	async function confirm() {
		submitError = '';
		status = '';
		if (
			!shouldPostStamp({
				intent: 'confirm',
				targets,
				selectedNodeId,
				lat,
				lon
			})
		) {
			return;
		}
		if (mock) {
			status = 'Stamped (preview).';
			return;
		}
		const id = identityStore.identity;
		if (!id || lat == null || lon == null) {
			submitError = 'Create an identity first, then stamp.';
			return;
		}
		busy = true;
		try {
			const altVal = parseCoord(altInput);
			await submitCoordinateStamp({
				secretKey: id.secretKey,
				publicKey: id.publicKey,
				nodeId: selectedNodeId,
				lat,
				lon,
				alt: altVal ?? undefined
			});
			status = 'Stamp sent. Last-known will show after the node ingests it.';
		} catch (err) {
			submitError = err instanceof Error ? err.message : String(err);
		} finally {
			busy = false;
		}
	}
</script>

{#if targets.length === 0}
	<p class="text-sm text-muted-foreground">No nearby routers to stamp.</p>
{:else}
	<div class="flex flex-col gap-3">
		<div class="flex items-center gap-2 text-sm font-medium">
			<MapPinned class="size-4 text-muted-foreground" aria-hidden="true" />
			Stamp location
		</div>
		<p class="text-sm text-muted-foreground">
			Standing next to a router? Pick it (you are on the first one) and confirm GPS from this phone.
		</p>

		<fieldset class="flex flex-col gap-1">
			<legend class="sr-only">Router to stamp</legend>
			{#each targets as t (t.nodeId)}
				<label
					class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted {selectedNodeId ===
					t.nodeId
						? 'bg-muted'
						: ''}"
				>
					<input type="radio" name="stamp-node" class="accent-primary" bind:group={selectedNodeId} value={t.nodeId} />
					{#if t.name}
						<span class="font-medium">{t.name}</span>
					{:else}
						<span class="text-muted-foreground italic">{t.fallback}</span>
					{/if}
					{#if t.isSelf}
						<span class="text-xs text-primary">you are here</span>
					{/if}
					{#if t.signalDbm !== undefined}
						<span class="font-mono text-xs text-muted-foreground">{t.signalDbm} dBm</span>
					{/if}
					{#if t.lat !== undefined && t.lon !== undefined}
						<span class="ml-auto font-mono text-xs text-muted-foreground">{formatCoord(t.lat, t.lon)}</span>
					{/if}
				</label>
			{/each}
		</fieldset>

		{#if selected?.lat !== undefined && selected?.lon !== undefined}
			<p class="text-xs text-muted-foreground">
				Last-known: {formatCoord(selected.lat, selected.lon)}
				{#if selected.stamper}
					· {selected.stamper.slice(0, 8)}…
				{/if}
			</p>
		{/if}

		<div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
			<label class="flex flex-col gap-1 text-xs text-muted-foreground">
				Latitude
				<input class={inputClass} type="text" inputmode="decimal" autocomplete="off" bind:value={latInput} placeholder="37.77490" />
			</label>
			<label class="flex flex-col gap-1 text-xs text-muted-foreground">
				Longitude
				<input class={inputClass} type="text" inputmode="decimal" autocomplete="off" bind:value={lonInput} placeholder="-122.41940" />
			</label>
			<label class="flex flex-col gap-1 text-xs text-muted-foreground">
				Altitude m <span class="font-normal">(optional)</span>
				<input class={inputClass} type="text" inputmode="decimal" autocomplete="off" bind:value={altInput} placeholder="" />
			</label>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<Button variant="outline" size="sm" onclick={useGps} disabled={gpsBusy}>
				<LocateFixed data-icon="inline-start" />
				{gpsBusy ? 'Locating…' : 'Use GPS'}
			</Button>
			<Button variant="ghost" size="sm" onclick={cancel}>Cancel</Button>
			<Button size="sm" onclick={confirm} disabled={!canConfirm}>
				{busy ? 'Stamping…' : 'Confirm stamp'}
			</Button>
		</div>

		{#if !mock && !identityStore.identity}
			<p class="text-xs text-muted-foreground">Create an identity above — the stamp is signed by your phone key.</p>
		{/if}
		{#if gpsError}
			<p class="text-xs text-muted-foreground">{gpsError}</p>
		{/if}
		{#if submitError}
			<p class="text-xs text-destructive">{submitError}</p>
		{/if}
		{#if status}
			<p class="text-xs text-primary">{status}</p>
		{/if}
	</div>
{/if}
