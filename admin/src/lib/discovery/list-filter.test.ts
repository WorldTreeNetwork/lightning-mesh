import { describe, expect, it } from 'vitest';
import {
	DEFAULT_FILTERS,
	PRESETS,
	ifaceSelectionLabel,
	matchingPreset,
	nextIfaceSelection,
	resourceMatches,
	stateBucket,
	type ListFilters
} from './list-filter';

function row(state: string, extra: Partial<{ meshName: string | null; router: boolean }> = {}) {
	return { state, meshName: extra.meshName ?? null, router: extra.router ?? false };
}

describe('nearby list filters', () => {
	it('hides stale and failed until those filters are on', () => {
		expect(resourceMatches(row('reachable'), DEFAULT_FILTERS)).toBe(true);
		expect(resourceMatches(row('noarp'), DEFAULT_FILTERS)).toBe(true);
		expect(resourceMatches(row('stale'), DEFAULT_FILTERS)).toBe(false);
		expect(resourceMatches(row('failed'), DEFAULT_FILTERS)).toBe(false);
		expect(resourceMatches(row('incomplete'), DEFAULT_FILTERS)).toBe(false);
	});

	it('shows a bucket only when that filter is selected', () => {
		const staleOn: ListFilters = { ...DEFAULT_FILTERS, live: false, stale: true };
		expect(resourceMatches(row('stale'), staleOn)).toBe(true);
		expect(resourceMatches(row('reachable'), staleOn)).toBe(false);
		expect(stateBucket('stale')).toBe('stale');
		expect(stateBucket('down')).toBe('failed');
		expect(stateBucket('reachable')).toBe('live');
	});

	it('keeps an unknown state with the live set', () => {
		expect(stateBucket('weird')).toBe('live');
		expect(resourceMatches(row('weird'), DEFAULT_FILTERS)).toBe(true);
	});

	it('presets name the useful cuts', () => {
		expect(matchingPreset(DEFAULT_FILTERS)).toBe('all');
		const lightning = PRESETS.find((preset) => preset.id === 'lightning')!.filters;
		expect(resourceMatches(row('reachable', { meshName: 'ap3000' }), lightning)).toBe(true);
		expect(resourceMatches(row('stale', { meshName: 'ap3000' }), lightning)).toBe(false);
		expect(resourceMatches(row('reachable'), lightning)).toBe(false);
		expect(resourceMatches({ ...row('reachable'), lightning: true }, lightning)).toBe(true);
		const problems = PRESETS.find((preset) => preset.id === 'problems')!.filters;
		expect(resourceMatches(row('stale'), problems)).toBe(true);
		expect(resourceMatches(row('failed'), problems)).toBe(true);
		expect(resourceMatches(row('reachable'), problems)).toBe(false);
		expect(matchingPreset({ ...DEFAULT_FILTERS, stale: true })).toBeNull();
	});

	it('selects one interface, and ctrl toggles', () => {
		expect(nextIfaceSelection([], 'wlan0', false)).toEqual(['wlan0']);
		expect(nextIfaceSelection(['wlan0'], 'wlan0', false)).toEqual([]);
		expect(nextIfaceSelection(['wlan0'], 'eth0', true)).toEqual(['wlan0', 'eth0']);
		expect(nextIfaceSelection(['wlan0', 'eth0'], 'wlan0', true)).toEqual(['eth0']);
		expect(ifaceSelectionLabel([])).toBe('All interfaces');
		expect(ifaceSelectionLabel(['wlan0', 'eth0'])).toBe('2 interfaces');
	});

	it('narrows to lightning nodes and gateways together', () => {
		const both: ListFilters = { ...DEFAULT_FILTERS, lightning: true, gateway: true };
		expect(resourceMatches(row('reachable', { meshName: 'ap3000', router: true }), both)).toBe(true);
		expect(resourceMatches(row('reachable', { meshName: 'ap3000' }), both)).toBe(false);
		expect(resourceMatches(row('reachable', { router: true }), both)).toBe(false);
		expect(resourceMatches(row('stale', { meshName: 'ap3000', router: true }), both)).toBe(false);
	});
});
