import { expect, test, type ConsoleMessage, type Page, type Request } from '@playwright/test';

// Guards the failure this bead exists for: on a real mesh node the page is
// served over plain HTTP at hello.mesh, which is NOT a secure context, so
// `crypto.subtle` is absent. Identity must still work there.
//
// Everything here runs against the shipped adapter-static bundle served by the
// real mjolnir-hello binary (see playwright.config.ts `webServer`), so a broken
// build — wrong MIME types, a module that never loads, a blank hydrate — fails
// the test instead of quietly shipping.

/** Fail loudly on anything that would leave the user staring at a blank page. */
function trackPageFailures(page: Page): string[] {
	const failures: string[] = [];

	page.on('requestfailed', (req: Request) => {
		failures.push(`requestfailed ${req.url()} — ${req.failure()?.errorText ?? 'unknown'}`);
	});
	page.on('response', (res) => {
		if (res.status() >= 400) failures.push(`HTTP ${res.status()} ${res.url()}`);
	});
	page.on('pageerror', (err: Error) => {
		failures.push(`pageerror ${err.message}`);
	});
	page.on('console', (msg: ConsoleMessage) => {
		if (msg.type() !== 'error') return;
		failures.push(`console.error ${msg.text()}`);
	});

	return failures;
}

test('hello.mesh works over an insecure origin, without crypto.subtle', async ({ page }) => {
	const failures = trackPageFailures(page);

	await page.goto('/');

	// Tripwire. If this ever reports true, the test is talking to localhost (or
	// https) and no longer proves anything — the whole suite is then a lie.
	const context = await page.evaluate(() => ({
		origin: window.location.origin,
		isSecureContext: window.isSecureContext,
		subtle: typeof globalThis.crypto?.subtle
	}));
	console.log(
		`[smoke] origin=${context.origin} isSecureContext=${context.isSecureContext} ` +
			`crypto.subtle=${context.subtle}`
	);
	expect(context.origin).toBe(new URL(page.url()).origin);
	expect(context.origin.startsWith('http://hello.mesh')).toBe(true);
	expect(context.isSecureContext).toBe(false);
	expect(context.subtle).toBe('undefined');

	// The identity panel lives behind the hero chip. Both the chip's anonymous
	// label and the panel opening on click are hydration-only: a bundle that
	// never ran leaves the prerendered title and nothing else.
	const chip = page.getByRole('button', { name: 'Set up your identity' });
	await expect(chip).toBeVisible();
	await chip.click();

	const nameInput = page.locator('#identity-name');
	await expect(nameInput).toBeVisible();
	await expect(page.getByText('Introduce yourself')).toBeVisible();

	expect(failures, `page failures before identity:\n${failures.join('\n')}`).toEqual([]);

	// The real ceremony: keygen + challenge signing, then POST /api/identity
	// against the live spool. All of it without WebCrypto.
	const submitted = page.waitForResponse(
		(res) => res.url().includes('/api/identity') && res.request().method() === 'POST'
	);
	await nameInput.fill('Smoke Tester');
	await page.getByRole('button', { name: 'Join' }).click();

	const response = await submitted;
	expect(response.status(), await response.text()).toBeLessThan(300);

	// Post-join UI: the anonymous form is replaced by the rename form.
	await expect(page.locator('#identity-rename')).toBeVisible();
	await expect(page.locator('#identity-rename')).toHaveValue('Smoke Tester');

	expect(failures, `page failures after identity:\n${failures.join('\n')}`).toEqual([]);
});
