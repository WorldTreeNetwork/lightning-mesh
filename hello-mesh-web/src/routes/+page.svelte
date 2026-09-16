<!-- hello.mesh front desk, one page. Top to bottom: an orientation hero that
     names this router and carries your identity (create/rename/reveal/export/
     import behind an expander, custody notice included — bead c3f); People on
     the network; the Services you can open (above the router topology — it's how
     walk-up users find apps like KEYED, so it leads and opens by default,
     mjolnir-mesh-kgq); and the Routers that make up the mesh + their radio links.
     One shared /api/directory poll (directoryStore) backs People, Services, and
     Routers; the identity store rides the same poll to derive your status live.
     No external hosts — must run fully offline. -->
<script lang="ts">
	import HeroStrip from '$lib/components/HeroStrip.svelte';
	import PeoplePanel from '$lib/components/PeoplePanel.svelte';
	import RoutersPanel from '$lib/components/RoutersPanel.svelte';
	import AppsPanel from '$lib/components/AppsPanel.svelte';
	import ServicesPanel from '$lib/components/ServicesPanel.svelte';
	import { directoryStore, startDirectoryPolling } from '$lib/directory/store.svelte';
	import { startIdentity } from '$lib/identity/store.svelte';
	import { isMiniApp } from '$lib/miniapp/contract';
	import { isFramed } from '$lib/miniapp/identity-bridge';
	import { browser } from '$app/environment';

	const framed = browser && isFramed(window);

	$effect(startDirectoryPolling);
	$effect(() => {
		if (framed) return;
		return startIdentity();
	});

	const services = $derived(directoryStore.directory?.services ?? []);
	const ordinary = $derived(services.filter((svc) => !isMiniApp(svc)));
</script>

<main class="mx-auto flex max-w-2xl flex-col gap-6 p-6">
	<HeroStrip />

	<PeoplePanel />

	<AppsPanel {services} loaded={directoryStore.loaded} />

	<ServicesPanel services={ordinary} loaded={directoryStore.loaded} />

	<RoutersPanel />
</main>
