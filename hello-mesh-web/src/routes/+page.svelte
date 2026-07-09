<!-- hello.mesh front desk, one page. Top to bottom: an orientation hero that
     names this router and carries your identity (create/rename/reveal/export/
     import behind an expander, custody notice included — bead c3f); People on
     the network; the Routers that make up the mesh + their radio links; and the
     Services you can open. One shared /api/directory poll (directoryStore) backs
     People, Routers, and Services; the identity store rides the same poll to
     derive your status live. No external hosts — must run fully offline. -->
<script lang="ts">
	import HeroStrip from '$lib/components/HeroStrip.svelte';
	import PeoplePanel from '$lib/components/PeoplePanel.svelte';
	import RoutersPanel from '$lib/components/RoutersPanel.svelte';
	import ServicesPanel from '$lib/components/ServicesPanel.svelte';
	import { directoryStore, startDirectoryPolling } from '$lib/directory/store.svelte';
	import { startIdentity } from '$lib/identity/store.svelte';

	$effect(startDirectoryPolling);
	$effect(startIdentity);

	const services = $derived(directoryStore.directory?.services ?? []);
</script>

<main class="mx-auto flex max-w-2xl flex-col gap-6 p-6">
	<HeroStrip />

	<PeoplePanel />

	<RoutersPanel />

	<ServicesPanel {services} loaded={directoryStore.loaded} />
</main>
