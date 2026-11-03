<script lang="ts">
	import { onMount } from 'svelte';
	import { guildSnapshot } from '$lib/guild/api';
	import AppearanceMenu from './AppearanceMenu.svelte';
	import SectionBar from './SectionBar.svelte';

	let { children } = $props();
	let guildName = $state('Guild');

	onMount(() => {
		void guildSnapshot()
			.then((snapshot) => {
				const named =
					snapshot.guilds.find((guild) => guild.workplace && guild.name) ??
					snapshot.guilds.find((guild) => guild.profile !== 'access-node' && guild.name) ??
					snapshot.guilds.find((guild) => guild.name);
				if (named?.name) guildName = named.name;
			})
			.catch(() => {
				/* The dev page has no Tauri host. The switcher fills this in later. */
			});
	});
</script>

<div class="app">
	<header class="header">
		<div class="guild">
			<span class="kicker">Guild</span>
			<span>{guildName}</span>
		</div>
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

	.guild {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-medium);
		line-height: var(--line-height-tight);
	}

	.kicker {
		color: var(--muted);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-regular);
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
