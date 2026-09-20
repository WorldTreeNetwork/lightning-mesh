import { navBySection } from '$lib/server/docs';
import { SECTION_LABELS, SECTION_ORDER } from '$lib/paths';

export async function load() {
	const grouped = await navBySection();
	const sections = SECTION_ORDER.filter((key) => grouped[key]?.length).map((key) => ({
		key,
		label: SECTION_LABELS[key] ?? key,
		items: grouped[key] ?? []
	}));
	return { sections };
}
