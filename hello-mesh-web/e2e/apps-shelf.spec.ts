import { expect, test } from '@playwright/test';

test('Apps shelf: no iframe or app-host hints on load; mini-apps leave Services', async ({
	page
}) => {
	await page.goto('/');

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
	const appLink = page.getByRole('link', { name: 'Open', exact: true });
	await expect(appLink).toBeVisible();
	await expect(appLink).toHaveAttribute('href', 'http://keyed.mesh:3000/app');
	await expect(page.getByRole('link', { name: 'library' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'keyed' })).toHaveCount(0);
	await expect(page.locator('iframe')).toHaveCount(0);
});
