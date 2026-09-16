<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';

	let {
		origin,
		appName,
		displayName,
		shortKey,
		offerCreate,
		onallow,
		ondeny,
		ondismiss
	}: {
		origin: string;
		appName?: string;
		displayName: string;
		shortKey: string;
		offerCreate: boolean;
		onallow: () => void;
		ondeny: () => void;
		ondismiss: () => void;
	} = $props();
</script>

<div
	class="fixed inset-0 z-50 grid place-items-center bg-background/80 p-4"
	role="dialog"
	aria-modal="true"
	aria-labelledby="consent-title"
>
	<section class="w-full max-w-md rounded-lg border border-border bg-card p-5 text-card-foreground">
		{#if offerCreate}
			<h2 id="consent-title" class="text-lg font-semibold">Create your identity first</h2>
			<p class="mt-2 text-sm text-muted-foreground">
				<span class="font-medium text-foreground">{origin}</span>
				{#if appName}
					<span> (calls itself {appName})</span>
				{/if}
				wants to know who you are.
			</p>
			<div class="mt-4 flex gap-2">
				<a
					href="#identity"
					class="inline-flex h-8 items-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground"
					onclick={ondismiss}
				>
					Create your identity first
				</a>
				<Button size="sm" variant="outline" onclick={ondismiss}>Not now</Button>
			</div>
		{:else}
			<h2 id="consent-title" class="text-lg font-semibold">{origin} wants to know you</h2>
			<p class="mt-1 text-sm text-muted-foreground">
				{#if appName}
					calls itself {appName}
				{/if}
			</p>
			<p class="mt-3 text-sm">
				You are <span class="font-medium">{displayName || 'unnamed'}</span>
				<code class="ml-1 font-mono text-xs text-muted-foreground">{shortKey}</code>
			</p>
			<div class="mt-4 flex gap-2">
				<Button size="sm" onclick={onallow}>Allow</Button>
				<Button size="sm" variant="outline" onclick={ondeny}>Deny</Button>
				<Button size="sm" variant="ghost" onclick={ondismiss}>Dismiss</Button>
			</div>
		{/if}
	</section>
</div>
