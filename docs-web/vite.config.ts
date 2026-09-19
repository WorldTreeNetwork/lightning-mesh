import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				fallback: '404.html',
				precompress: false,
				strict: true
			}),
			prerender: {
				handleHttpError: 'warn',
				handleMissingId: 'warn'
			}
		})
	],
	test: {
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,ts}']
	},
	server: {
		fs: {
			allow: ['..']
		}
	}
});
