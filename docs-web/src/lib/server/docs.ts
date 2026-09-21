import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { parse as parseYaml } from 'yaml';
import { marked, type Tokens } from 'marked';
import GithubSlugger from 'github-slugger';
import {
	navKey,
	relToSlug,
	rewriteDocHref,
	sectionOf,
	shouldSkip,
	titleFromBody,
	type DocMeta,
	type DocRecord
} from '$lib/paths';

const DOCS_ROOT = join(process.cwd(), 'content');

async function walk(dir: string, prefix = ''): Promise<string[]> {
	const entries = await readdir(dir, { withFileTypes: true });
	const out: string[] = [];
	for (const e of entries) {
		const rel = prefix ? `${prefix}/${e.name}` : e.name;
		if (e.isDirectory()) {
			out.push(...(await walk(join(dir, e.name), rel)));
		} else {
			out.push(rel);
		}
	}
	return out;
}

function splitFrontmatter(raw: string): { meta: DocMeta; body: string } {
	if (!raw.startsWith('---\n') && !raw.startsWith('---\r\n')) {
		return { meta: {}, body: raw };
	}
	const end = raw.indexOf('\n---', 4);
	if (end < 0) return { meta: {}, body: raw };
	const yamlBlock = raw.slice(4, end);
	const body = raw.slice(end + 4).replace(/^\r?\n/, '');
	let meta: DocMeta = {};
	try {
		const parsed = parseYaml(yamlBlock);
		if (parsed && typeof parsed === 'object') meta = parsed as DocMeta;
	} catch {
		meta = {};
	}
	return { meta, body };
}

function renderMarkdown(body: string, fromRel: string): string {
	const slugger = new GithubSlugger();
	const renderer = new marked.Renderer();
	renderer.link = ({ href, title, text }: Tokens.Link) => {
		const next = rewriteDocHref(href ?? '', fromRel);
		const t = title ? ` title="${escapeHtml(title)}"` : '';
		return `<a href="${escapeHtml(next)}"${t}>${text}</a>`;
	};
	renderer.heading = ({ text, depth }: Tokens.Heading) => {
		const id = slugger.slug(text);
		return `<h${depth} id="${id}"><a class="heading-anchor" href="#${id}">${text}</a></h${depth}>`;
	};
	return marked.parse(body, { renderer, gfm: true, async: false }) as string;
}

function escapeHtml(s: string): string {
	return s
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;');
}

let cache: DocRecord[] | null = null;

export async function loadDocs(): Promise<DocRecord[]> {
	if (cache) return cache;
	const files = (await walk(DOCS_ROOT)).filter((rel) => !shouldSkip(rel));
	const docs: DocRecord[] = [];
	for (const rel of files) {
		const raw = await readFile(join(DOCS_ROOT, rel), 'utf8');
		const { meta, body } = splitFrontmatter(raw);
		const slug = relToSlug(rel);
		const title = meta.title || titleFromBody(body, slug || 'Lightning Mesh');
		docs.push({
			slug,
			rel,
			meta,
			title,
			section: sectionOf(slug),
			body: renderMarkdown(body, rel)
		});
	}
	docs.sort((a, b) => a.slug.localeCompare(b.slug));
	cache = docs;
	return docs;
}

export async function getDoc(slug: string): Promise<DocRecord | undefined> {
	const docs = await loadDocs();
	return docs.find((d) => d.slug === slug);
}

export async function getDocById(id: string): Promise<DocRecord | undefined> {
	const docs = await loadDocs();
	return docs.find((d) => d.meta.id === id);
}

export type NavItem = { slug: string; title: string; order: number; status?: string };

export async function navBySection(): Promise<Record<string, NavItem[]>> {
	const docs = await loadDocs();
	const grouped: Record<string, NavItem[]> = {};
	for (const d of docs) {
		const key = navKey(d.slug, d.meta.path);
		(grouped[key] ??= []).push({
			slug: d.slug,
			title: d.title,
			order: d.meta.order ?? 999,
			status: d.meta.status
		});
	}
	for (const items of Object.values(grouped)) {
		items.sort((a, b) => a.order - b.order || a.slug.localeCompare(b.slug));
	}
	return grouped;
}
