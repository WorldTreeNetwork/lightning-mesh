<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { ifaceSelectionLabel, nextIfaceSelection } from './list-filter';

	let {
		groups,
		selected,
		showAll,
		hiddenCount,
		onSelect,
		onShowAll
	}: {
		groups: { kind: string; label: string; ifaces: { name: string; is_up: boolean }[] }[];
		selected: string[];
		showAll: boolean;
		hiddenCount: number;
		onSelect: (selected: string[]) => void;
		onShowAll: () => void;
	} = $props();

	const label = $derived(ifaceSelectionLabel(selected));
	let multi = $state(false);

	function arm(event: PointerEvent) {
		multi = event.ctrlKey || event.metaKey;
	}

	function pick(name: string, event: Event) {
		if (multi) event.preventDefault();
		onSelect(nextIfaceSelection(selected, name, multi));
		multi = false;
	}

	function all(event: Event) {
		if (multi) event.preventDefault();
		onSelect([]);
		multi = false;
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<button type="button" class="textlink" {...props}>{label}</button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
		<DropdownMenu.Content class="lm-menu iface-menu" align="start" sideOffset={4} onpointerdowncapture={arm}>
			<DropdownMenu.Item class="lm-menu-item" onSelect={all}>
				<span class="lm-menu-mark">{selected.length === 0 ? '●' : ''}</span>
				All interfaces
			</DropdownMenu.Item>
			{#each groups as group (group.kind)}
				<DropdownMenu.Group>
					<DropdownMenu.GroupHeading class="lm-menu-label">{group.label}</DropdownMenu.GroupHeading>
					{#each group.ifaces as iface (iface.name)}
						<DropdownMenu.Item
							class="lm-menu-item"
							onSelect={(event) => pick(iface.name, event)}
						>
							<span class="lm-menu-mark">{selected.includes(iface.name) ? '●' : ''}</span>
							<span class="grow" class:down={!iface.is_up}>{iface.name}</span>
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Group>
			{/each}
			<DropdownMenu.Separator class="lm-menu-sep" />
			<p class="hint">Ctrl-click to select more than one.</p>
			{#if hiddenCount > 0 || showAll}
				<DropdownMenu.CheckboxItem class="lm-menu-item" checked={showAll} closeOnSelect={false} onCheckedChange={onShowAll}>
					{#snippet children({ checked })}
						<span class="lm-menu-mark">{checked ? '●' : ''}</span>
						Show bridges and virtual
					{/snippet}
				</DropdownMenu.CheckboxItem>
			{/if}
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
		max-width: 14rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.textlink:hover {
		text-decoration: underline;
	}

	:global(.iface-menu) {
		min-width: 14rem;
		max-height: 22rem;
		overflow: auto;
	}

	.grow {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.down {
		color: var(--muted);
	}

	.hint {
		margin: 0.15rem 0.5rem 0.35rem;
		color: var(--muted);
		font-size: 12px;
	}
</style>
