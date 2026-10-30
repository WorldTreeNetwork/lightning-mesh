import { invoke } from '@tauri-apps/api/core';

/** Open the single canvas overlay, or focus it if already open. */
export async function openCanvas(): Promise<void> {
	await invoke('open_canvas');
}
