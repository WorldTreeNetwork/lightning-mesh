import { posix as path } from 'node:path';

export type DocMeta = {
	id?: string;
	title?: string;
	description?: string;
	path?: string;
	order?: number;
	audience?: string[];
	status?: string;
	time?: string;
	requires?: string[];
	next_step?: string | null;
	verified_against?: string;
};

export type DocRecord = {
	/** URL slug without leading slash, e.g. `join/person/01-connect` */
	slug: string;
	/** Path relative to `docs-web/content/`, e.g. `join/person/01-connect.md` */
	rel: string;
	meta: DocMeta;
	title: string;
	section: string;
	body: string;
};

export function shouldSkip(rel: string): boolean {
	return !rel.endsWith('.md');
}

export function relToSlug(rel: string): string {
	const noExt = rel.replace(/\.md$/, '');
	if (noExt.endsWith('/index') || noExt === 'index') {
		return noExt.replace(/\/?index$/, '') || '';
	}
	if (noExt.endsWith('/README') || noExt === 'README') {
		return noExt.replace(/\/?README$/, '') || '';
	}
	return noExt;
}

export function sectionOf(slug: string): string {
	if (!slug) return 'home';
	return slug.split('/')[0] ?? 'home';
}

export function titleFromBody(body: string, fallback: string): string {
	const m = body.match(/^#\s+(.+)$/m);
	return m?.[1]?.trim() || fallback;
}

/** Rewrite a markdown href that points at another doc file. */
export function rewriteDocHref(href: string, fromRel: string): string {
	if (!href || href.startsWith('#') || href.startsWith('mailto:')) return href;
	if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return href;

	const hashIdx = href.indexOf('#');
	const hash = hashIdx >= 0 ? href.slice(hashIdx) : '';
	const bare = hashIdx >= 0 ? href.slice(0, hashIdx) : href;
	if (!bare.endsWith('.md') && !bare.includes('.md')) return href;

	const fromDir = path.dirname(fromRel);
	const resolved = path.normalize(path.join(fromDir === '.' ? '' : fromDir, bare));
	const cleaned = resolved.replace(/^\.\//, '');
	const relMd = cleaned.endsWith('.md') ? cleaned : `${cleaned}.md`;
	if (relMd.startsWith('../')) {
		const repoRel = relMd.replace(/^(\.\.\/)+/, '');
		return `https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/${repoRel}${hash}`;
	}
	const slug = relToSlug(relMd);
	return `/${slug}${hash}`;
}

export function navKey(slug: string, pathField?: string): string {
	if (pathField && pathField !== 'index') return pathField;
	const parts = slug.split('/');
	if (parts[0] === 'join' && parts[1]) return parts[1];
	return 'start';
}

export const SECTION_LABELS: Record<string, string> = {
	start: 'Start',
	person: 'Use the mesh',
	house: 'Set up a home',
	node: 'Build a router',
	publish: 'Share a service',
	contribute: 'Contribute'
};

export const SECTION_ORDER = ['start', 'person', 'house', 'node', 'publish', 'contribute'];
