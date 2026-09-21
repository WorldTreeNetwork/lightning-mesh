<script lang="ts">
	import './layout.css';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import StatusChip from '$lib/components/StatusChip.svelte';

	let { children, data } = $props();
	let open = $state(false);

	const current = $derived(page.url.pathname.replace(/^\/|\/$/g, '') || 'join');
	const activeSection = $derived(current.split('/')[1] || 'start');
</script>

<svelte:head>
	<title>Lightning Mesh docs</title>
	<meta name="description" content="Join, operate, and extend a Lightning Mesh network." />
</svelte:head>

<a href="#main" class="skip">Skip to content</a>

<div class="flex min-h-screen flex-col lg:flex-row">
	<header class="bg-bg-2 lg:hidden">
		<div class="hazard" aria-hidden="true"></div>
		<div class="flex items-center justify-between px-4 py-3">
			<a
				href={resolve('/')}
				class="font-display text-sm tracking-widest text-[var(--color-bolt)] uppercase"
				>Lightning Mesh</a
			>
			<button
				type="button"
				class="border-border size-9 rounded-sm border text-[var(--color-ink-muted)]"
				aria-expanded={open}
				aria-label="Menu"
				onclick={() => (open = !open)}
			>
				{open ? '✕' : '☰'}
			</button>
		</div>
	</header>

	<aside
		class="border-border bg-bg-2 w-full shrink-0 border-b lg:sticky lg:top-0 lg:h-screen lg:w-72 lg:overflow-y-auto lg:border-r lg:border-b-0 {open
			? 'block'
			: 'hidden lg:block'}"
	>
		<div class="hazard" aria-hidden="true"></div>
		<div class="px-5 py-6">
			<a href={resolve('/')} class="block">
				<p class="font-display text-[0.7rem] tracking-[0.28em] text-[var(--color-bolt)]">
					LIGHTNING MESH
				</p>
				<p
					class="font-hud mt-2 text-[0.65rem] tracking-[0.22em] text-[var(--color-ink-muted)] uppercase"
				>
					User manual
				</p>
			</a>
		</div>
		<nav class="px-3 pb-10" aria-label="User guide">
			{#each data.sections as section (section.key)}
				<details class="nav-section mb-2" open={section.key === activeSection}>
					<summary
						class="font-hud cursor-pointer rounded-sm px-2 py-2 text-[0.65rem] tracking-widest text-[var(--color-ink-muted)] uppercase hover:text-[var(--color-bolt)]"
					>
						{section.label}
					</summary>
					<ul class="mt-1 border-l border-[var(--color-border)] pl-2">
						{#each section.items as item (item.slug)}
							<li>
								<a
									href={resolve('/[...slug]', { slug: item.slug })}
									class="flex items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-sm {current ===
									item.slug
										? 'bg-bg text-[var(--color-bolt)]'
										: 'text-[var(--color-ink)] hover:text-[var(--color-bolt)]'}"
									onclick={() => (open = false)}
								>
									<span class="truncate">{item.title}</span>
									<StatusChip status={item.status} />
								</a>
							</li>
						{/each}
					</ul>
				</details>
			{/each}
		</nav>
	</aside>

	<main id="main" class="min-w-0 flex-1 px-5 py-8 sm:px-10">
		{@render children()}
	</main>
</div>

<style>
	.skip {
		position: absolute;
		left: 0.5rem;
		top: 0.5rem;
		z-index: 100;
		padding: 0.4rem 0.8rem;
		background: var(--color-bolt);
		color: #111;
		transform: translateY(-160%);
	}
	.skip:focus {
		transform: translateY(0);
	}
	.nav-section summary::-webkit-details-marker {
		display: none;
	}
	.nav-section summary::before {
		content: '›';
		display: inline-block;
		margin-right: 0.5rem;
		transition: transform 120ms ease;
	}
	.nav-section[open] summary::before {
		transform: rotate(90deg);
	}
</style>
