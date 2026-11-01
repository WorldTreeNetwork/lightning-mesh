import { invoke } from '@tauri-apps/api/core';

/** Open a new canvas well. Rejects visibly at the caller when four are open. */
export async function openCanvas(): Promise<void> {
	await invoke('open_canvas');
}
