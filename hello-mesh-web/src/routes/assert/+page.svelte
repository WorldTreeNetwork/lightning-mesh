<!--
	hello.mesh identity-assertion handoff (bead mjolnir-mesh-1f2).

	A relying party (`wiki.mesh`, …) redirects the browser here to get a signed,
	self-certifying assertion of the visitor's rung-1 identity, OIDC-style — the
	RP never touches the browser-held key, it just receives a token in the return
	URL FRAGMENT and verifies it offline (see verifyAssertion in
	$lib/identity/assert). Consent is remembered per audience in localStorage so a
	prompt=none re-auth can complete silently.

	Client-only: reads window.location, IndexedDB (the key), and localStorage.
	The prerendered shell renders the neutral "Checking…" state; all real work
	runs in onMount behind the `browser` guard.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { Button } from '$lib/components/ui/button/index.js';
	import { publicKeyHex } from '$lib/identity/keys';
	import { loadIdentity, type StoredIdentity } from '$lib/identity/storage';
	import {
		parseAssertRequest,
		buildAssertionPayload,
		signAssertion,
		encodeAssertionToken,
		buildReturnUrl,
		buildErrorUrl,
		type AssertRequest,
		type AssertErrorCode
	} from '$lib/identity/assert';

	const APPROVALS_KEY = 'hello-mesh-assert-approvals';

	type View = 'checking' | 'invalid' | 'no-identity' | 'consent' | 'redirecting';

	let view = $state<View>('checking');
	let request = $state<AssertRequest | undefined>(undefined);
	let identity = $state<StoredIdentity | undefined>(undefined);
	let errorDetail = $state('');
	let busy = $state(false);

	const displayName = $derived(identity?.label?.trim() || '');
	const pubkey = $derived(identity ? publicKeyHex(identity.publicKey) : '');
	const shortKey = $derived(pubkey ? pubkey.slice(0, 8) : '');

	function loadApprovals(): Record<string, number> {
		try {
			const raw = localStorage.getItem(APPROVALS_KEY);
			return raw ? (JSON.parse(raw) as Record<string, number>) : {};
		} catch {
			return {};
		}
	}

	function isApproved(audience: string): boolean {
		return Boolean(loadApprovals()[audience]);
	}

	function rememberApproval(audience: string) {
		try {
			const approvals = loadApprovals();
			approvals[audience] = Math.floor(Date.now() / 1000);
			localStorage.setItem(APPROVALS_KEY, JSON.stringify(approvals));
		} catch {
			// Approval memory is a convenience; ignore storage failures.
		}
	}

	/** Sign an assertion for the current request and redirect back to the RP. */
	function completeApproval(req: AssertRequest, id: StoredIdentity) {
		const payloadJson = buildAssertionPayload({
			pubkey: publicKeyHex(id.publicKey),
			displayName: id.label?.trim() || '',
			audience: req.audience,
			nonce: req.nonce,
			issuedAt: Math.floor(Date.now() / 1000)
		});
		const sig = signAssertion(id.secretKey, payloadJson);
		const token = encodeAssertionToken({ payload: payloadJson, sig });
		redirect(buildReturnUrl(req.returnTo, token));
	}

	function fail(req: AssertRequest, code: AssertErrorCode) {
		redirect(buildErrorUrl(req.returnTo, code));
	}

	function redirect(url: string) {
		view = 'redirecting';
		// Replace so the handoff page isn't left in the RP's history/back stack.
		window.location.replace(url);
	}

	function approve() {
		if (!request || !identity || busy) return;
		busy = true;
		rememberApproval(request.audience);
		completeApproval(request, identity);
	}

	function deny() {
		if (!request) return;
		fail(request, 'access_denied');
	}

	onMount(async () => {
		if (!browser) return;

		const parsed = parseAssertRequest(new URLSearchParams(window.location.search));
		if ('error' in parsed) {
			// Can't safely redirect (return_to/audience are the untrusted, invalid
			// inputs), so surface the error on-page.
			view = 'invalid';
			errorDetail =
				'The request was missing or malformed (audience, return_to, and a hex nonce are required, and return_to must live at the audience origin).';
			return;
		}
		request = parsed;
		identity = await loadIdentity();

		if (parsed.prompt === 'none') {
			// Silent path: only complete if we already have a key AND prior consent.
			if (identity && isApproved(parsed.audience)) {
				completeApproval(parsed, identity);
			} else {
				fail(parsed, 'interaction_required');
			}
			return;
		}

		// prompt=consent: always show the consent screen (or the create-first panel).
		view = identity ? 'consent' : 'no-identity';
	});
</script>

<main class="mx-auto flex min-h-svh max-w-md flex-col justify-center gap-6 p-6">
	{#if view === 'checking' || view === 'redirecting'}
		<section class="flex flex-col items-center gap-2 py-12 text-center">
			<p class="text-sm text-muted-foreground">
				{view === 'redirecting' ? 'Signing you in…' : 'Checking…'}
			</p>
		</section>
	{:else if view === 'invalid'}
		<section
			class="flex flex-col gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-5 text-card-foreground"
		>
			<h1 class="text-lg font-semibold">This sign-in request is invalid</h1>
			<p class="text-sm text-muted-foreground">{errorDetail}</p>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href="/" class="text-sm font-medium underline underline-offset-2">Back to hello.mesh</a>
		</section>
	{:else if view === 'no-identity'}
		<section
			class="flex flex-col gap-3 rounded-lg border border-border bg-card p-5 text-card-foreground"
		>
			<h1 class="text-lg font-semibold">Create your identity first</h1>
			<p class="text-sm text-muted-foreground">
				{request?.audience} wants to know who you are, but you don't have an identity on this mesh
				yet. Create one on the front desk, then come back to this page to continue.
			</p>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a
				href="/"
				class="inline-flex h-9 w-fit items-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary/90"
			>
				Go to the front desk
			</a>
			<p class="text-xs text-muted-foreground">
				After you've introduced yourself, return here (or have {request?.audience} send you back) to
				finish signing in.
			</p>
		</section>
	{:else if view === 'consent'}
		<section
			class="flex flex-col gap-4 rounded-lg border border-border bg-card p-5 text-card-foreground"
		>
			<div class="flex flex-col gap-1.5">
				<h1 class="text-lg font-semibold">Sign in to {request?.audience}?</h1>
				<p class="text-sm text-muted-foreground">
					<span class="font-medium text-foreground">{request?.audience}</span> will learn your public
					key and display name.
				</p>
			</div>

			<div class="flex flex-col gap-1 rounded-md border border-border bg-muted/40 p-3">
				<div class="text-sm">
					<span class="text-muted-foreground">You are</span>
					<span class="font-medium">{displayName || 'unnamed'}</span>
				</div>
				<code class="font-mono text-xs text-muted-foreground">{shortKey}… </code>
			</div>

			<div class="flex gap-2">
				<Button size="sm" disabled={busy} onclick={approve}>Approve</Button>
				<Button size="sm" variant="outline" disabled={busy} onclick={deny}>Deny</Button>
			</div>
			<p class="text-xs text-muted-foreground">
				Only your public key and name are shared — never your private key or recovery phrase.
			</p>
		</section>
	{/if}
</main>
