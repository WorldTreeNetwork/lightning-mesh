import { getDoc, getDocById, loadDocs } from '$lib/server/docs';
import { error } from '@sveltejs/kit';

export async function entries() {
	const docs = await loadDocs();
	return docs.filter((d) => d.slug).map((d) => ({ slug: d.slug }));
}

export async function load({ params }) {
	const slug = params.slug.replace(/\/$/, '');
	const doc = await getDoc(slug);
	if (!doc) error(404, `No document at ${slug}`);
	const nextId = doc.meta.next_step;
	const next = nextId ? await getDocById(nextId) : undefined;
	return {
		doc,
		next: next ? { slug: next.slug, title: next.title } : null
	};
}
