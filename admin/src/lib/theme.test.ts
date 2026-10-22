import { describe, expect, test } from 'vitest';
import { resolveVariant } from './theme';

describe('theme variants', () => {
	test('light and dark stay put', () => {
		expect(resolveVariant('light')).toBe('light');
		expect(resolveVariant('dark')).toBe('dark');
	});
});
