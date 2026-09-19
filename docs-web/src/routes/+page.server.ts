import { getDoc } from '$lib/server/docs';
import { error } from '@sveltejs/kit';

export async function load() {
	const doc = (await getDoc('join')) ?? (await getDoc(''));
	if (!doc) error(404, 'Join guide not found');
	return { doc };
}
