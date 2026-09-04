import { describe, expect, it } from 'vitest';
import { encodeBase58, ipv6ToBase58, parseIpv6Bytes } from './base58';

describe('ipv6 base58', () => {
	it('treats each hex couplet as a byte and encodes Bitcoin base58', () => {
		const cases: [string, string][] = [
			['fd01:d28c:7e4a::1', 'YF4RhMGBc3LA1xkSVuXzpg'],
			['fe80::1', 'YRka4zYGRkixTpb4LjCkzL'],
			['fe80::6dd0:82fe:420c:6779', 'YRka4zYGRkjGqACkk8onBa'],
			['fc00::', 'Y7r4v4m4eqstVx6aWDjQjH'],
			['::1', '1111111111111112']
		];
		for (const [ip, encoded] of cases) {
			expect(ipv6ToBase58(ip), ip).toBe(encoded);
		}
	});

	it('strips a %zone before encoding', () => {
		expect(ipv6ToBase58('fe80::1%eth0')).toBe('YRka4zYGRkixTpb4LjCkzL');
	});

	it('round-trips 16 zero bytes to sixteen 1s', () => {
		expect(encodeBase58(new Uint8Array(16))).toBe('1'.repeat(16));
	});

	it('rejects junk', () => {
		expect(parseIpv6Bytes('not-an-ip')).toBeNull();
		expect(ipv6ToBase58('192.168.0.1')).toBeNull();
	});
});
