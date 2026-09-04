/** Bitcoin base58 alphabet (no 0/O/I/l). */
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

/** Encode raw bytes as Bitcoin base58, preserving leading-zero bytes as `'1'`. */
export function encodeBase58(bytes: Uint8Array): string {
	let zeros = 0;
	while (zeros < bytes.length && bytes[zeros] === 0) zeros++;
	const acc = Array.from(bytes);
	const digits: number[] = [];
	while (acc.some((b) => b !== 0)) {
		let rem = 0;
		for (let i = 0; i < acc.length; i++) {
			const cur = (rem << 8) | acc[i];
			acc[i] = Math.floor(cur / 58);
			rem = cur % 58;
		}
		digits.push(rem);
	}
	return '1'.repeat(zeros) + digits.reverse().map((d) => ALPHABET[d]).join('');
}

/** Parse an IPv6 text form (optional `%zone`) into the 16-octet address. */
export function parseIpv6Bytes(addr: string): Uint8Array | null {
	const host = addr.split('%')[0]?.trim();
	if (!host || !host.includes(':')) return null;
	const compressed = host.includes('::');
	const [head, tail] = compressed ? host.split('::') : [host, undefined];
	const headParts = head ? head.split(':') : [];
	const tailParts = tail ? tail.split(':') : [];
	if (headParts.some((p) => p.includes('.')) || tailParts.some((p) => p.includes('.'))) {
		return null;
	}
	const filled = compressed ? 8 - headParts.length - tailParts.length : 0;
	if (compressed) {
		if (filled < 0) return null;
	} else if (headParts.length !== 8) {
		return null;
	}
	const parts = compressed
		? [...headParts, ...Array(filled).fill('0'), ...tailParts]
		: headParts;
	if (parts.length !== 8) return null;
	const bytes = new Uint8Array(16);
	for (let i = 0; i < 8; i++) {
		if (!/^[0-9a-fA-F]{0,4}$/.test(parts[i])) return null;
		const n = Number.parseInt(parts[i] || '0', 16);
		if (!Number.isFinite(n) || n < 0 || n > 0xffff) return null;
		bytes[i * 2] = (n >> 8) & 0xff;
		bytes[i * 2 + 1] = n & 0xff;
	}
	return bytes;
}

/** Encode an IPv6 address (hex couplets → bytes) as base58. */
export function ipv6ToBase58(addr: string): string | null {
	const bytes = parseIpv6Bytes(addr);
	if (!bytes) return null;
	return encodeBase58(bytes);
}
