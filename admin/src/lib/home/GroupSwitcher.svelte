<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { ChevronDown } from '@lucide/svelte';
	import { groupResources } from '$lib/discovery/resources';
	import { discovery } from '$lib/discovery/store.svelte';
	import { heardRouters, localTiles, present, selectionCount } from './groups';
	import { groupSelection } from './selection.svelte';

	const resources = $derived(groupResources(discovery.neighbors));
	const view = $derived(
		present(
			groupSelection.groups,
			groupSelection.selectedId,
			heardRouters(resources),
			localTiles(resources)
		)
	);
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<button type="button" class="trigger" {...props}>
				<span>{view.selected.name}</span>
				<ChevronDown size={14} strokeWidth={1.75} aria-hidden="true" />
			</button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
		<DropdownMenu.Content class="lm-menu" align="start" sideOffset={4}>
			<DropdownMenu.RadioGroup
				value={view.selected.id}
				onValueChange={(value) => groupSelection.select(value)}
			>
				{#each view.list as row (row.group.id)}
					<DropdownMenu.RadioItem class="lm-menu-item group-row" value={row.group.id}>
						{#snippet children({ checked })}
							<span class="lm-menu-mark" aria-hidden="true">{checked ? '●' : ''}</span>
							<span class="name">{row.group.name}</span>
							<span class="count">{selectionCount(row.group, row.count)}</span>
						{/snippet}
					</DropdownMenu.RadioItem>
				{/each}
			</DropdownMenu.RadioGroup>
		</DropdownMenu.Content>
	</DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
	.trigger {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		min-height: 28px;
		padding: var(--space-1) var(--space-2);
		border: 0;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-medium);
	}

	.trigger:hover {
		background: var(--row-hover);
	}

	:global(.group-row) {
		justify-content: flex-start;
		gap: var(--space-2);
	}

	.name {
		flex: 1;
	}

	.count {
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
</style>
