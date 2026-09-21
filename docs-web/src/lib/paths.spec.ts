import { describe, expect, it } from 'vitest';
import { relToSlug, rewriteDocHref, shouldSkip } from './paths';

describe('relToSlug', () => {
	it('maps join index to join', () => {
		expect(relToSlug('join/index.md')).toBe('join');
	});
	it('maps nested pages', () => {
		expect(relToSlug('join/person/01-connect.md')).toBe('join/person/01-connect');
	});
	it('maps README to folder', () => {
		expect(relToSlug('storage-node/README.md')).toBe('storage-node');
	});
});

describe('rewriteDocHref', () => {
	it('rewrites a sibling markdown link', () => {
		expect(rewriteDocHref('02-hello-mesh.md', 'join/person/01-connect.md')).toBe(
			'/join/person/02-hello-mesh'
		);
	});
	it('rewrites a parent-relative link', () => {
		expect(rewriteDocHref('../index.md', 'join/person/01-connect.md')).toBe('/join');
	});
	it('keeps hashes', () => {
		expect(rewriteDocHref('../index.md#what-works-today', 'join/person/01-connect.md')).toBe(
			'/join#what-works-today'
		);
	});
	it('leaves http links alone', () => {
		expect(rewriteDocHref('https://example.com/foo.md', 'join/index.md')).toBe(
			'https://example.com/foo.md'
		);
	});
	it('sends non-user docs to GitHub', () => {
		expect(rewriteDocHref('../../../docs/vision/why-decentralized-mesh.md', 'join/index.md')).toBe(
			'https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/docs/vision/why-decentralized-mesh.md'
		);
	});
});

describe('shouldSkip', () => {
	it('accepts Markdown only because the content tree is the publication boundary', () => {
		expect(shouldSkip('join/index.md')).toBe(false);
		expect(shouldSkip('join/person/01-connect.md')).toBe(false);
		expect(shouldSkip('join/llms.txt')).toBe(true);
		expect(shouldSkip('join/router.png')).toBe(true);
	});
});
