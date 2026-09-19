import { posix as path } from 'node:path';

const SKIP_PREFIXES = [
	'archive/',
	'sprints/',
	'storage-node/consults/',
	'research/openwrt-mesh-hardware/hypotheses/',
	'research/openwrt-mesh-hardware/pass2/',
	'research/openwrt-mesh-hardware/pass3-budget/'
];

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
	/** Path relative to `docs/`, e.g. `join/person/01-connect.md` */
	rel: string;
	meta: DocMeta;
	title: string;
	section: string;
	body: string;
};

export function shouldSkip(rel: string): boolean {
	if (!rel.endsWith('.md')) return true;
	if (rel.endsWith('LEARNINGS.md')) return true;
	return SKIP_PREFIXES.some((p) => rel.startsWith(p));
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
	const slug = relToSlug(cleaned.endsWith('.md') ? cleaned : `${cleaned}.md`);
	return `/${slug}${hash}`;
}

export const SECTION_LABELS: Record<string, string> = {
	join: 'Join',
	vision: 'Vision',
	'network-coordination': 'Architecture',
	deploy: 'Deploy',
	products: 'Products',
	'storage-node': 'Storage node',
	research: 'Research',
	transfer: 'Transfer'
};
