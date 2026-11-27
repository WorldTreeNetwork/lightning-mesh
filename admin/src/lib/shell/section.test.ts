import { describe, expect, it } from 'vitest';
import { sectionFor } from './section';

describe('sectionFor', () => {
	it('maps each section path', () => {
		expect(sectionFor('/')).toBe('home');
		expect(sectionFor('/backhaul')).toBe('backhaul');
		expect(sectionFor('/guild')).toBe('guild');
		expect(sectionFor('/group')).toBe('guild');
		expect(sectionFor('/map')).toBe('map');
		expect(sectionFor('/network')).toBe('network');
		expect(sectionFor('/nearby')).toBe('network');
		expect(sectionFor('/services')).toBe('services');
		expect(sectionFor('/you')).toBe('you');
	});

	it('keeps the opener marked on pages that are not sections', () => {
		expect(sectionFor('/router/abc')).toBe('home');
		expect(sectionFor('/host/host%3Apixel.local')).toBe('home');
		expect(sectionFor('/you/become')).toBe('you');
		expect(sectionFor('/become')).toBe('you');
	});
});
