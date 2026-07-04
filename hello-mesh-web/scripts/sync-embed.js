#!/usr/bin/env node
// Copies the SvelteKit static build output into crates/mjolnir-hello/static/,
// which the Rust server embeds at compile time via rust-embed. Run via
// `npm run build:embed` (after `npm run build`, or invoked together by that
// script).
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const buildDir = path.join(here, '..', 'build');
const embedDir = path.join(here, '..', '..', 'crates', 'mjolnir-hello', 'static');

if (!existsSync(buildDir)) {
	console.error(`build output not found at ${buildDir} — run \`npm run build\` first`);
	process.exit(1);
}

mkdirSync(embedDir, { recursive: true });
// Clear out any previous synced bundle before copying the fresh one, so
// stale files from an earlier build don't linger in the embedded crate.
rmSync(embedDir, { recursive: true, force: true });
mkdirSync(embedDir, { recursive: true });
cpSync(buildDir, embedDir, { recursive: true });

console.log(`synced ${buildDir} -> ${embedDir}`);
