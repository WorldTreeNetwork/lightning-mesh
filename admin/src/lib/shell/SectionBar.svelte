<script lang="ts">
	import { page } from '$app/state';
	import { CircleUser, House, Map, PanelLeftClose, PanelLeftOpen, Radio, Shield, Waypoints, Wifi } from '@lucide/svelte';
	import { sectionFor, type SectionId } from './section';

	const STORAGE_KEY = 'lightning-admin-nav-collapsed';

	const items: { id: SectionId; href: string; label: string; icon: typeof House }[] = [
		{ id: 'home', href: '/', label: 'Home', icon: House },
		{ id: 'backhaul', href: '/backhaul', label: 'Backhaul', icon: Wifi },
		{ id: 'guild', href: '/guild', label: 'Guild', icon: Shield },
		{ id: 'map', href: '/map', label: 'Map', icon: Map },
		{ id: 'nearby', href: '/nearby', label: 'Nearby', icon: Radio },
		{ id: 'services', href: '/services', label: 'Services', icon: Waypoints }
	];

	let collapsed = $state(false);

	$effect(() => {
		try {
			collapsed = localStorage.getItem(STORAGE_KEY) === '1';
		} catch {
			/* private mode */
		}
	});

	const current = $derived(sectionFor(page.url.pathname));

	function toggle() {
		collapsed = !collapsed;
		try {
			localStorage.setItem(STORAGE_KEY, collapsed ? '1' : '0');
		} catch {
			/* private mode */
		}
	}
</script>

<nav class="bar" class:collapsed aria-label="Sections">
	<div class="sections">
		{#each items as item (item.id)}
			{@const Icon = item.icon}
			<a
				href={item.href}
				aria-current={current === item.id ? 'page' : undefined}
			>
				<Icon size={16} strokeWidth={1.75} aria-hidden="true" />
				<span class:clip={collapsed}>{item.label}</span>
			</a>
		{/each}
	</div>
	<div class="tail">
		<a href="/you" aria-current={current === 'you' ? 'page' : undefined}>
			<CircleUser size={16} strokeWidth={1.75} aria-hidden="true" />
			<span class:clip={collapsed}>You</span>
		</a>
		<button type="button" onclick={toggle} aria-pressed={collapsed}>
			{#if collapsed}
				<PanelLeftOpen size={16} strokeWidth={1.75} aria-hidden="true" />
			{:else}
				<PanelLeftClose size={16} strokeWidth={1.75} aria-hidden="true" />
			{/if}
			<span class:clip={collapsed}>{collapsed ? 'Expand sections' : 'Collapse sections'}</span>
		</button>
	</div>
</nav>

<style>
	.bar {
		display: flex;
		flex-direction: column;
		width: 168px;
		flex: none;
		background: var(--surface);
		border-right: 1px solid var(--border);
	}

	.bar.collapsed {
		width: 40px;
	}

	.sections {
		display: flex;
		flex-direction: column;
		padding: var(--space-1);
		gap: var(--space-1);
	}

	.tail {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-top: auto;
		padding: var(--space-1);
		border-top: 1px solid var(--border);
	}

	a,
	button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-height: 28px;
		padding: var(--space-1) var(--space-2);
		border: 0;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font-size: var(--font-size-sm);
		text-align: left;
		text-decoration: none;
		letter-spacing: var(--letter-spacing-normal);
		text-transform: none;
	}

	.collapsed a,
	.collapsed button {
		justify-content: center;
		padding-inline: 0;
		gap: 0;
	}

	a:hover,
	button:hover {
		background: var(--row-hover);
	}

	a[aria-current='page'] {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.clip {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
