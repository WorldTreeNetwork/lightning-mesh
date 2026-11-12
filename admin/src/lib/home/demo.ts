import type { SavedGroup } from './groups';

/** Browser preview (`?demo=1`) with a roster the demo scan only partly hears. */
export function demoGroups(): SavedGroup[] {
	return [
		{
			id: 'house',
			name: 'Strawberry Creek',
			you: 'admin',
			assigned: [
				{ id: 'aa', name: 'ap3000' },
				{ id: 'bb', name: 'tr3000' },
				{ id: 'cc', name: 'mt3000' },
				{ id: 'dd', name: 'spare' },
				{ id: 'ee', name: 'loft' }
			]
		},
		{
			id: 'shop',
			name: 'Shop',
			you: 'member',
			assigned: [{ id: 'bb', name: 'tr3000' }]
		}
	];
}
