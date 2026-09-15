import { expect, test } from '@playwright/test';

test('Apps shelf: no iframe or app-host hints on load; mini-apps leave Services', async ({
	page
}) => {
	await page.goto('/?mockDirectory=1');

	await expect(page.getByText('No apps on this mesh yet.')).toHaveCount(0);
	await expect(page.getByText('Apps', { exact: true })).toBeVisible();
	await expect(page.locator('iframe')).toHaveCount(0);

	const leakingLinks = page.locator(
		'link[rel="preconnect"], link[rel="prefetch"], link[rel="dns-prefetch"], link[rel="icon"]'
	);
	const hrefs = await leakingLinks.evaluateAll((nodes) =>
		nodes.map((n) => (n as HTMLLinkElement).href)
	);
	expect(hrefs.every((href) => !href.includes('keyed.mesh'))).toBe(true);

	await expect(page.getByText('keyed.mesh:3000')).toBeVisible();
	await expect(page.getByRole('link', { name: 'Open' }).first()).toBeVisible();
	await expect(page.locator('iframe')).toHaveCount(0);
});
