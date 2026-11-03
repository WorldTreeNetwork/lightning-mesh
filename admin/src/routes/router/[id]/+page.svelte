<script lang="ts">
	import { page } from '$app/state';
	import StubPage from '$lib/shell/StubPage.svelte';

	const title = $derived(page.params.id ? `Router ${page.params.id}` : 'Router');
</script>

<StubPage {title} backHref="/" />
