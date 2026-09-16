/** Shared audience approvals for `/assert` and the mini-app identity bridge. */
export const APPROVALS_KEY = 'hello-mesh-assert-approvals';

export function loadApprovals(storage: Pick<Storage, 'getItem'> | null): Record<string, number> {
	if (!storage) return {};
	try {
		const raw = storage.getItem(APPROVALS_KEY);
		return raw ? (JSON.parse(raw) as Record<string, number>) : {};
	} catch {
		return {};
	}
}

export function isApproved(
	storage: Pick<Storage, 'getItem'> | null,
	audience: string
): boolean {
	return Boolean(loadApprovals(storage)[audience]);
}

export function rememberApproval(
	storage: Pick<Storage, 'getItem' | 'setItem'> | null,
	audience: string,
	nowUnix = Math.floor(Date.now() / 1000)
): void {
	if (!storage) return;
	try {
		const approvals = loadApprovals(storage);
		approvals[audience] = nowUnix;
		storage.setItem(APPROVALS_KEY, JSON.stringify(approvals));
	} catch {
		// Convenience only.
	}
}
