<!--
	Apps shelf: mini-apps from the directory, decorated by GET /api/apps.
	No iframe until tap. One card at a time. Closing removes the frame and listener.
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import type { DirectoryService } from '$lib/directory/api';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { LayoutGrid, ExternalLink, X } from '@lucide/svelte';
	import {
		FRAME_ALLOW,
		FRAME_REFERRER,
		FRAME_SANDBOX,
		acceptBridgeMessage,
		canEmbed,
		clampHeight,
		entryOrigin,
		entryUrl,
		safeOpenUrl,
		sendBridgeMessage
	} from '$lib/miniapp/contract';
	import {
		bindCardMessages,
		fetchAppsJson,
		shelfTiles,
		tileIsLinkOut,
		tileManifest,
		type ShelfTile
	} from '$lib/miniapp/apps';

	let { services, loaded = true }: { services: DirectoryService[]; loaded?: boolean } = $props();

	let appsJson = $state<unknown | null>(null);
	let open = $state(true);
	let opened = $state<string | null>(null);
	let frameEl = $state<HTMLIFrameElement | null>(null);
	let cardHeight = $state(320);
	let inited = $state(false);

	const tiles = $derived(shelfTiles(services, appsJson));
	const count = $derived(tiles.length);
	const pageOrigin = $derived(browser ? window.location.origin : 'http://hello.mesh');

	$effect(() => {
		if (!browser) return;
		let cancelled = false;
		const load = () => {
			fetchAppsJson()
				.then((json) => {
					if (!cancelled) appsJson = json;
				})
				.catch(() => {
					if (!cancelled) appsJson = null;
				});
		};
		load();
		const interval = setInterval(load, 5000);
		return () => {
			cancelled = true;
			clearInterval(interval);
		};
	});

	function tileKey(tile: ShelfTile): string {
		const s = tile.service;
		return `${s.name}|${s.protocol}|${s.ip}|${s.port}`;
	}

	function displayName(tile: ShelfTile): string {
		return tileManifest(tile)?.name ?? tile.service.name.replace(/\._[a-z]+\._[a-z]+$/i, '');
	}

	function hostLabel(tile: ShelfTile): string {
		const url = entryUrl(tile.service);
		try {
			return url ? new URL(url).host : tile.service.ip;
		} catch {
			return tile.service.ip;
		}
	}

	function openCard(tile: ShelfTile) {
		const url = entryUrl(tile.service);
		if (!url) return;
		if (
			tileIsLinkOut(tile) ||
			!canEmbed(url, pageOrigin, tileManifest(tile)?.embed)
		) {
			return;
		}
		opened = tileKey(tile);
		cardHeight = tileManifest(tile)?.height ?? 320;
		inited = false;
	}

	function closeCard() {
		opened = null;
		frameEl = null;
		inited = false;
	}

	$effect(() => {
		if (!browser || !opened || !frameEl) return;
		const frame = frameEl;
		const tile = tiles.find((t) => tileKey(t) === opened);
		const origin = tile ? entryOrigin(tile.service) : undefined;
		if (!tile || !origin) return;
		const onMessage = (event: MessageEvent) => {
			const msg = acceptBridgeMessage(event, frame.contentWindow, origin);
			if (!msg) return;
			if (msg.type === 'ready' && !inited && frame.contentWindow) {
				inited = true;
				sendBridgeMessage(frame.contentWindow, { mesh: 'mini-app/v1', type: 'init', v: 1 }, origin);
			} else if (msg.type === 'resize') {
				cardHeight = clampHeight(msg.height);
			} else if (msg.type === 'open') {
				const url = safeOpenUrl(msg.url);
				if (url) window.open(url, '_blank', 'noopener');
			}
		};
		return bindCardMessages(window, onMessage);
	});
</script>

{#if !loaded}
	<p class="text-sm text-muted-foreground">Loading apps…</p>
{:else if count === 0}
	<p class="text-sm text-muted-foreground">No apps on this mesh yet.</p>
{:else}
	<Collapsible.Root bind:open class="rounded-lg border border-border bg-card text-card-foreground">
		<Collapsible.Trigger
			class="flex w-full items-center gap-3 rounded-lg px-4 py-3 text-left transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
		>
			<LayoutGrid class="size-4 text-muted-foreground" aria-hidden="true" />
			<span class="flex-1">
				<span class="font-semibold">Apps</span>
				<span class="ml-1 text-sm text-muted-foreground">tap to open</span>
			</span>
			<Badge variant="default">{count}</Badge>
		</Collapsible.Trigger>
		<Collapsible.Content>
			<ul class="flex flex-col gap-3 border-t border-border px-4 py-3">
				{#each tiles as tile (tileKey(tile))}
					{@const url = entryUrl(tile.service)}
					{@const manifest = tileManifest(tile)}
					{@const embeddable =
						!!url && !tileIsLinkOut(tile) && canEmbed(url, pageOrigin, manifest?.embed)}
					{@const isOpen = opened === tileKey(tile)}
					<li class="rounded-md border border-border bg-background/40 p-3">
						<div class="flex items-start gap-3">
							{#if manifest?.icon}
								<img src={manifest.icon} alt="" class="size-10 rounded-md object-cover" />
							{/if}
							<div class="min-w-0 flex-1">
								<p class="font-medium">{displayName(tile)}</p>
								<p class="text-xs text-muted-foreground">{hostLabel(tile)}</p>
								{#if manifest?.description}
									<p class="mt-1 text-sm text-muted-foreground">{manifest.description}</p>
								{/if}
								{#if tile.record?.stale && manifest}
									<p class="mt-1 text-xs text-muted-foreground">May be out of date</p>
								{/if}
							</div>
							{#if url}
								<a
									href={url}
									target="_blank"
									rel="noopener noreferrer"
									class="inline-flex items-center gap-1 text-sm text-primary hover:underline"
								>
									Open
									<ExternalLink class="size-3" aria-hidden="true" />
								</a>
							{/if}
						</div>
						{#if embeddable && !isOpen}
							<Button class="mt-2" size="sm" variant="secondary" onclick={() => openCard(tile)}>
								Open here
							</Button>
						{/if}
						{#if isOpen && url}
							<div class="mt-3">
								<div class="mb-2 flex justify-end">
									<Button size="xs" variant="ghost" onclick={closeCard}>
										<X class="size-3" aria-hidden="true" />
										Close
									</Button>
								</div>
								<iframe
									bind:this={frameEl}
									src={url}
									title={displayName(tile)}
									sandbox={FRAME_SANDBOX}
									allow={FRAME_ALLOW}
									referrerpolicy={FRAME_REFERRER}
									style="height: {cardHeight}px"
									class="w-full rounded-md border border-border bg-background"
								></iframe>
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		</Collapsible.Content>
	</Collapsible.Root>
{/if}
