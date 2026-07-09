// Copy-to-clipboard that survives an insecure context. hello.mesh is plain
// HTTP, so `navigator.clipboard` is often undefined; fall back to a hidden
// textarea + execCommand('copy'), which still works without a secure origin.

export async function copyText(text: string): Promise<boolean> {
	try {
		if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
			await navigator.clipboard.writeText(text);
			return true;
		}
	} catch {
		// fall through to the legacy path
	}
	if (typeof document === 'undefined') return false;
	try {
		const ta = document.createElement('textarea');
		ta.value = text;
		ta.setAttribute('readonly', '');
		ta.style.position = 'fixed';
		ta.style.opacity = '0';
		ta.style.pointerEvents = 'none';
		document.body.appendChild(ta);
		ta.select();
		const ok = document.execCommand('copy');
		document.body.removeChild(ta);
		return ok;
	} catch {
		return false;
	}
}
