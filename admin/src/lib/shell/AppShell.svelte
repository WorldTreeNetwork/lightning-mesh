<script lang="ts">
	import { onMount } from 'svelte';
	import { discovery } from '$lib/discovery/store.svelte';
	import GroupSwitcher from '$lib/home/GroupSwitcher.svelte';
	import { groupSelection } from '$lib/home/selection.svelte';
	import AppearanceMenu from './AppearanceMenu.svelte';
	import SectionBar from './SectionBar.svelte';

	let { children } = $props();

	onMount(() => {
		void groupSelection.load();
		if (!discovery.lastScan && !discovery.loading) void discovery.scan();
	});
</script>

<div class="app">
	<header class="header">
		<GroupSwitcher />
		<AppearanceMenu />
	</header>
	<div class="body">
		<SectionBar />
		<main>
			{@render children()}
		</main>
	</div>
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 100%;
		background: var(--bg);
		color: var(--text);
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		min-height: 36px;
		padding: var(--space-1) var(--space-3);
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}

	.body {
		display: flex;
		flex: 1;
		min-height: 0;
	}

	main {
		flex: 1;
		min-width: 0;
		overflow: auto;
	}
</style>
