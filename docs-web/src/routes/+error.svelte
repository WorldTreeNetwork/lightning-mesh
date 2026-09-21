<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
</script>

<h1 class="font-display text-2xl tracking-widest uppercase">{page.status}</h1>
<p class="mt-2 text-[var(--color-ink-muted)]">{page.error?.message ?? 'Not found'}</p>
<p class="mt-6">
	<a class="text-[var(--color-link)]" href={resolve('/')}>Back to the join guide</a>
</p>
