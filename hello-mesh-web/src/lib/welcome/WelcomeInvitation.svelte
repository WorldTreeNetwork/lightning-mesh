<!--
	House-hub welcome invitation. UI-local: no choice here touches routing,
	firewall, CAPPORT, or the identity store except the explicit create form
	reached after consent. Local information (Services etc.) renders on the page
	regardless of this component's state.
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import { identityStore } from '$lib/identity/store.svelte';
	import ConsentSheet from '$lib/components/ConsentSheet.svelte';
	import IdentityManager from '$lib/components/IdentityManager.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import {
		initialInvitation,
		reduceInvitation,
		mayCreateIdentity,
		internetCopy,
		dismissCopy,
		storageVerdict,
		regularBrowserLimit,
		regularBrowserPath,
		type Action,
		type StorageProbe,
		type Upstream
	} from './welcome';

	let { upstream = 'unknown' }: { upstream?: Upstream } = $props();

	let invite = $state(initialInvitation);
	let probe = $state<StorageProbe | undefined>(undefined);

	function send(action: Action) {
		invite = reduceInvitation(invite, action);
	}

	async function runProbe() {
		if (!browser) return;
		let hasIndexedDb = typeof indexedDB !== 'undefined';
		let writeReadOk = false;
		try {
			localStorage.setItem('hm-probe', '1');
			writeReadOk = localStorage.getItem('hm-probe') === '1';
			localStorage.removeItem('hm-probe');
		} catch {
			writeReadOk = false;
		}
		probe = {
			userAgent: navigator.userAgent,
			hasIndexedDb,
			writeReadOk,
			portalHint: window.self !== window.top
		};
	}
	$effect(() => {
		void runProbe();
	});

	const unverified = $derived(storageVerdict(probe) === 'unverified');
	const paths = $derived(regularBrowserPath());
</script>

{#if !identityStore.identity}
	<section
		class="flex flex-col gap-3 rounded-lg border border-border bg-card p-5 text-card-foreground"
	>
		{#if invite.phase === 'invited' || invite.phase === 'consenting'}
			<h2 class="text-base font-semibold">Welcome. Look around, or introduce yourself.</h2>
			<p class="text-sm text-muted-foreground">
				Everything below works without an identity. Creating one is optional and never required to
				browse.
			</p>
			<div class="flex flex-wrap gap-2">
				<Button size="sm" variant="outline" onclick={() => send('explore')}
					>Explore anonymously</Button
				>
				<Button size="sm" onclick={() => send('startCreate')}>Create an identity</Button>
				<Button size="sm" variant="ghost" onclick={() => send('justInternet')}>
					Just the internet, please
				</Button>
				<Button size="sm" variant="ghost" onclick={() => send('dismiss')}>Dismiss</Button>
			</div>
		{:else if invite.phase === 'internet'}
			<p class="text-sm">{internetCopy(upstream)}</p>
		{:else if invite.phase === 'dismissed'}
			<p class="text-sm text-muted-foreground">{dismissCopy}</p>
		{:else if invite.phase === 'anonymous'}
			<p class="text-sm text-muted-foreground">
				Browsing anonymously. Nothing was created. Return any time at
				{#each paths as url, i (url)}{i ? ' or ' : ' '}<code class="font-mono text-xs">{url}</code
					>{/each}.
			</p>
		{:else if mayCreateIdentity(invite)}
			{#if unverified}
				<p class="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning">
					{regularBrowserLimit} For an identity that lasts, open this page in your regular browser at
					{#each paths as url, i (url)}{i ? ' or ' : ' '}<code class="font-mono text-xs">{url}</code
						>{/each}
					and create it there.
				</p>
			{/if}
			<IdentityManager allowRestore={!unverified} />
		{/if}
	</section>
{/if}

{#if invite.phase === 'consenting'}
	<ConsentSheet
		announce
		onallow={() => send('confirm')}
		ondeny={() => send('cancelConsent')}
		ondismiss={() => send('cancelConsent')}
	/>
{/if}
