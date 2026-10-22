<script lang="ts">
	import { onMount } from 'svelte';
	import '../app.css';
	import { applyAppearance, readAppearance } from '$lib/theme';

	let { children } = $props();

	onMount(() => {
		applyAppearance(readAppearance());
		const mq = window.matchMedia('(prefers-color-scheme: dark)');
		const onChange = () => {
			const current = readAppearance();
			if (current.variant === 'system') applyAppearance(current);
		};
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});
</script>

{@render children()}
