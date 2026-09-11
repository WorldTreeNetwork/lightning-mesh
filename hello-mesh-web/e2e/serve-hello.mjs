// Launch the real mjolnir-hello binary for the smoke test.
//
// Everything the daemon would normally write (directory/radio projections, the
// identity spool) is redirected into a throwaway temp dir, so the smoke never
// touches /var/run/mjolnir and a POST /api/identity can land for real.

import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const port = process.argv[2] ?? '8788';
const webRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const repoRoot = resolve(webRoot, '..');
const staticRoot = join(webRoot, 'build');

if (!existsSync(join(staticRoot, 'index.html'))) {
	console.error(`[serve-hello] missing ${staticRoot}/index.html — run \`bun run build\` first`);
	process.exit(1);
}

const targetDir = process.env.CARGO_TARGET_DIR ?? join(repoRoot, 'target');
const binary = join(targetDir, 'debug', 'mjolnir-hello');
if (!existsSync(binary)) {
	console.error(
		`[serve-hello] missing ${binary} — run \`cargo build -p mjolnir-hello\` first ` +
			`(set CARGO_TARGET_DIR to match)`
	);
	process.exit(1);
}

const state = mkdtempSync(join(tmpdir(), 'hello-smoke-'));
const spool = join(state, 'pending');
mkdirSync(spool, { recursive: true });
writeFileSync(
	join(state, 'directory.json'),
	JSON.stringify({ version: 1, node: null, neighbors: [], identities: [], services: [] })
);
writeFileSync(join(state, 'radio.json'), JSON.stringify({ version: 1 }));

console.log(`[serve-hello] state dir ${state}`);

const child = spawn(
	binary,
	[
		'--bind',
		`127.0.0.1:${port}`,
		'--static-root',
		staticRoot,
		'--directory-file',
		join(state, 'directory.json'),
		'--spool-dir',
		spool,
		'--radio-file',
		join(state, 'radio.json')
	],
	{ stdio: 'inherit', env: { ...process.env, HELLO_SMOKE_SPOOL: spool } }
);

const stop = () => child.kill('SIGTERM');
process.on('SIGTERM', stop);
process.on('SIGINT', stop);
child.on('exit', (code) => process.exit(code ?? 0));
