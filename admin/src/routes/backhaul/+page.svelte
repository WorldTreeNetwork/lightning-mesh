<script lang="ts">
	import NetworkNameBar from '$lib/network-name/NetworkNameBar.svelte';
</script>

<section class="page">
	<h1>Backhaul</h1>
	<p>
		This is the radio mesh. A phone keeps its address as it moves between access points on it.
		The nodes sync that lease and follow the phone with a host route. Roaming is not a smaller
		group inside the mesh, and a guild does not decide who is on the radio.
	</p>
	<NetworkNameBar />
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
