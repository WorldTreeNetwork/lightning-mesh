<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import {
		PRESETS,
		matchingPreset,
		type ListFilters,
		type PresetId
	} from './list-filter';

	let {
		filters,
		counts,
		onchange
	}: {
		filters: ListFilters;
		counts: Record<keyof ListFilters, number>;
		onchange: (filters: ListFilters) => void;
	} = $props();

	const preset = $derived(matchingPreset(filters));
	const label = $derived(PRESETS.find((item) => item.id === preset)?.label ?? 'Filtered');

	const checks: { key: keyof ListFilters; label: string }[] = [
		{ key: 'live', label: 'Live' },
		{ key: 'stale', label: 'Stale' },
		{ key: 'failed', label: 'Failed' },
		{ key: 'lightning', label: 'Lightning' },
		{ key: 'gateway', label: 'Gateways' }
	];

	function applyPreset(id: PresetId) {
		const next = PRESETS.find((item) => item.id === id);
		if (next) onchange({ ...next.filters });
	}

	function toggle(key: keyof ListFilters) {
		onchange({ ...filters, [key]: !filters[key] });
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<button type="button" class="textlink" {...props}>{label}</button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
		<DropdownMenu.Content class="lm-menu" align="start" sideOffset={4}>
			{#each PRESETS as item (item.id)}
				<DropdownMenu.Item class="lm-menu-item" onSelect={() => applyPreset(item.id)}>
					<span class="lm-menu-mark">{preset === item.id ? '●' : ''}</span>
					{item.label}
				</DropdownMenu.Item>
			{/each}
			<DropdownMenu.Separator class="lm-menu-sep" />
			{#each checks as item (item.key)}
				<DropdownMenu.CheckboxItem
					class="lm-menu-item"
					checked={filters[item.key]}
					closeOnSelect={false}
					onCheckedChange={() => toggle(item.key)}
				>
					{#snippet children({ checked })}
						<span class="lm-menu-mark">{checked ? '●' : ''}</span>
						<span class="grow">{item.label}</span>
						<span class="count">{counts[item.key]}</span>
					{/snippet}
				</DropdownMenu.CheckboxItem>
			{/each}
		</DropdownMenu.Content>
	</DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
	.textlink {
		border: 0;
		background: transparent;
		padding: 0.15rem 0.1rem;
		color: var(--accent);
		font-size: 13px;
		letter-spacing: 0;
		text-transform: none;
	}

	.textlink:hover {
		text-decoration: underline;
	}

	.grow {
		flex: 1;
	}

	.count {
		color: var(--muted);
		font-variant-numeric: tabular-nums;
		font-size: 12px;
	}
</style>
