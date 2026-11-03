<script lang="ts">
	import GuildPanel from '$lib/guild/mock/GuildPanel.svelte';
</script>

<section class="page">
	<h1>Guild</h1>
	<p>
		A guild records who owns a router and who may be admitted there. Joining the radio does not
		join the guild, and assigning a node does not change who that radio syncs with.
	</p>
	<GuildPanel />
</section>

<style>
	.page {
		display: flex;
		flex-direction: column;
		min-height: 100%;
	}

	h1 {
		margin: 0;
		padding: 0.8rem 1rem 0;
		font-size: 15px;
		font-weight: 600;
	}

	p {
		margin: 0.35rem 0 0.6rem;
		padding: 0 1rem;
		max-width: 42rem;
		color: var(--muted);
		font-size: 13px;
		line-height: 1.45;
	}
</style>
