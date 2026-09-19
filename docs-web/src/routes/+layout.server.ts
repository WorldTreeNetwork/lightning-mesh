import { navBySection } from '$lib/server/docs';
import { SECTION_LABELS } from '$lib/paths';

export async function load() {
	const grouped = await navBySection();
	const order = [
		'join',
		'vision',
		'network-coordination',
		'deploy',
		'products',
		'storage-node',
		'research',
		'transfer'
	];
	const sections = order
		.filter((key) => grouped[key]?.length)
		.map((key) => ({
			key,
			label: SECTION_LABELS[key] ?? key,
			items: grouped[key] ?? []
		}));
	return { sections };
}
