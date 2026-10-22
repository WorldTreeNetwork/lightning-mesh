export type ThemeName = 'plain' | 'cyber';
export type VariantChoice = 'system' | 'light' | 'dark';
export type ResolvedVariant = 'light' | 'dark';

export const THEME_NAMES: { id: ThemeName; label: string }[] = [
	{ id: 'plain', label: 'Plain' },
	{ id: 'cyber', label: 'Cyberpunk' }
];

export const VARIANT_CHOICES: { id: VariantChoice; label: string }[] = [
	{ id: 'system', label: 'System' },
	{ id: 'light', label: 'Light' },
	{ id: 'dark', label: 'Dark' }
];

const NAME_KEY = 'lightning-admin-theme-name';
const VARIANT_KEY = 'lightning-admin-variant';
/** Previous single control stored only the variant. */
const LEGACY_KEY = 'lightning-admin-theme';

export type Appearance = {
	name: ThemeName;
	variant: VariantChoice;
};

function isName(v: string | null): v is ThemeName {
	return v === 'plain' || v === 'cyber';
}

function isVariant(v: string | null): v is VariantChoice {
	return v === 'system' || v === 'light' || v === 'dark';
}

export function readAppearance(): Appearance {
	try {
		const name = localStorage.getItem(NAME_KEY);
		const variant = localStorage.getItem(VARIANT_KEY);
		if (isName(name) && isVariant(variant)) return { name, variant };
		const legacy = localStorage.getItem(LEGACY_KEY);
		if (isVariant(legacy)) return { name: 'plain', variant: legacy };
	} catch {
		/* private mode */
	}
	return { name: 'plain', variant: 'system' };
}

export function resolveVariant(choice: VariantChoice): ResolvedVariant {
	if (choice === 'light' || choice === 'dark') return choice;
	if (typeof matchMedia === 'undefined') return 'dark';
	return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function applyAppearance(appearance: Appearance): ResolvedVariant {
	const resolved = resolveVariant(appearance.variant);
	const root = document.documentElement;
	root.dataset.theme = appearance.name;
	root.dataset.variant = resolved;
	root.dataset.variantChoice = appearance.variant;
	try {
		localStorage.setItem(NAME_KEY, appearance.name);
		localStorage.setItem(VARIANT_KEY, appearance.variant);
		localStorage.removeItem(LEGACY_KEY);
	} catch {
		/* ignore */
	}
	return resolved;
}
