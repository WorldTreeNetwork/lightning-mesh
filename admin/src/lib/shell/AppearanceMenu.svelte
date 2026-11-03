<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import {
		applyAppearance,
		readAppearance,
		THEME_NAMES,
		VARIANT_CHOICES,
		type ThemeName,
		type VariantChoice
	} from '$lib/theme';

	let themeName = $state<ThemeName>('plain');
	let variantChoice = $state<VariantChoice>('system');

	const saved = readAppearance();
	themeName = saved.name;
	variantChoice = saved.variant;

	function setAppearance(next: { name?: ThemeName; variant?: VariantChoice }) {
		if (next.name) themeName = next.name;
		if (next.variant) variantChoice = next.variant;
		applyAppearance({ name: themeName, variant: variantChoice });
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<button type="button" class="appearance" {...props}>Appearance</button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
		<DropdownMenu.Content class="lm-menu" align="end" sideOffset={4}>
			<DropdownMenu.Group>
				<DropdownMenu.GroupHeading class="lm-menu-label">Theme</DropdownMenu.GroupHeading>
				<DropdownMenu.RadioGroup
					value={themeName}
					onValueChange={(value) => setAppearance({ name: value as ThemeName })}
				>
					{#each THEME_NAMES as theme (theme.id)}
						<DropdownMenu.RadioItem class="lm-menu-item" value={theme.id} closeOnSelect={false}>
							{#snippet children({ checked })}
								<span class="lm-menu-mark" aria-hidden="true">{checked ? '●' : ''}</span>
								{theme.label}
							{/snippet}
						</DropdownMenu.RadioItem>
					{/each}
				</DropdownMenu.RadioGroup>
			</DropdownMenu.Group>
			<DropdownMenu.Separator class="lm-menu-sep" />
			<DropdownMenu.Group>
				<DropdownMenu.GroupHeading class="lm-menu-label">Variant</DropdownMenu.GroupHeading>
				<DropdownMenu.RadioGroup
					value={variantChoice}
					onValueChange={(value) => setAppearance({ variant: value as VariantChoice })}
				>
					{#each VARIANT_CHOICES as variant (variant.id)}
						<DropdownMenu.RadioItem class="lm-menu-item" value={variant.id} closeOnSelect={false}>
							{#snippet children({ checked })}
								<span class="lm-menu-mark" aria-hidden="true">{checked ? '●' : ''}</span>
								{variant.label}
							{/snippet}
						</DropdownMenu.RadioItem>
					{/each}
				</DropdownMenu.RadioGroup>
			</DropdownMenu.Group>
		</DropdownMenu.Content>
	</DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
	.appearance {
		min-height: 28px;
		padding: var(--space-1) var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: transparent;
		color: var(--text);
		font-size: var(--font-size-sm);
	}

	.appearance:hover {
		background: var(--row-hover);
	}
</style>
