import { defineConfig } from '@playwright/test';

// hello.mesh smoke: the page MUST be loaded over a non-secure origin, because
// that is what a phone on the mesh actually gets. Browsers withhold
// `crypto.subtle` outside a secure context, so any identity code that assumes
// WebCrypto silently dies there. localhost is exempt from that rule — loading
// the test at 127.0.0.1 would hide the very bug this guards. So we resolve a
// real hostname to loopback inside Chromium instead.
const PORT = Number(process.env.HELLO_SMOKE_PORT ?? 8788);

export default defineConfig({
	testDir: 'e2e',
	// Keep failure artifacts out of the tree (node_modules is already ignored).
	outputDir: 'node_modules/.playwright-results',
	timeout: 60_000,
	expect: { timeout: 10_000 },
	fullyParallel: false,
	workers: 1,
	forbidOnly: !!process.env.CI,
	retries: 0,
	reporter: process.env.CI ? 'line' : 'list',
	use: {
		baseURL: `http://hello.mesh:${PORT}`,
		headless: true,
		// Chromium-only knob: send hello.mesh at loopback without touching
		// /etc/hosts, while the page origin stays the insecure hostname.
		launchOptions: {
			args: ['--host-resolver-rules=MAP hello.mesh 127.0.0.1']
		}
	},
	projects: [{ name: 'chromium' }],
	webServer: {
		// Real mjolnir-hello binary over the real adapter-static build — not a
		// vite dev server, so the smoke covers what the router actually ships.
		command: `node e2e/serve-hello.mjs ${PORT}`,
		// Readiness probe only; the browser never uses this origin.
		url: `http://127.0.0.1:${PORT}/api/health`,
		reuseExistingServer: false,
		stdout: 'pipe',
		stderr: 'pipe',
		timeout: 30_000
	}
});
