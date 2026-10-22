// Minimal QR encoder: byte mode, version 2, ECC level L, mask 0. Payload <= 32 bytes.
const SIZE = 25;
const DATA_WORDS = 34;
const ECC_WORDS = 10;

function rsEcc(data: number[]): number[] {
	const exp: number[] = [];
	const log: number[] = new Array(256).fill(0);
	let x = 1;
	for (let i = 0; i < 255; i++) {
		exp.push(x);
		log[x] = i;
		x <<= 1;
		if (x & 0x100) x ^= 0x11d;
	}
	const mul = (a: number, b: number) => (a && b ? exp[(log[a] + log[b]) % 255] : 0);
	let gen = [1];
	for (let i = 0; i < ECC_WORDS; i++) {
		const next = new Array(gen.length + 1).fill(0);
		for (let j = 0; j < gen.length; j++) {
			next[j] ^= gen[j];
			next[j + 1] ^= mul(gen[j], exp[i]);
		}
		gen = next;
	}
	const rem: number[] = new Array(ECC_WORDS).fill(0);
	for (const b of data) {
		const f = b ^ rem.shift()!;
		rem.push(0);
		for (let i = 0; i < ECC_WORDS; i++) rem[i] ^= mul(gen[i + 1], f);
	}
	return rem;
}

export function qrMatrix(text: string): boolean[][] {
	const bytes = Array.from(new TextEncoder().encode(text));
	if (bytes.length > 32) throw new Error('qr payload too long');

	const bits: number[] = [];
	const push = (v: number, n: number) => {
		for (let i = n - 1; i >= 0; i--) bits.push((v >>> i) & 1);
	};
	push(0b0100, 4);
	push(bytes.length, 8);
	for (const b of bytes) push(b, 8);
	push(0, Math.min(4, DATA_WORDS * 8 - bits.length));
	while (bits.length % 8) bits.push(0);
	const words: number[] = [];
	for (let i = 0; i < bits.length; i += 8) {
		words.push(bits.slice(i, i + 8).reduce((a, b) => (a << 1) | b, 0));
	}
	for (let pad = 0xec; words.length < DATA_WORDS; pad ^= 0xec ^ 0x11) words.push(pad);
	const all = [...words, ...rsEcc(words)];

	const m: boolean[][] = Array.from({ length: SIZE }, () => new Array(SIZE).fill(false));
	const fn: boolean[][] = Array.from({ length: SIZE }, () => new Array(SIZE).fill(false));
	const set = (x: number, y: number, dark: boolean) => {
		m[y][x] = dark;
		fn[y][x] = true;
	};

	for (let i = 0; i < SIZE; i++) {
		set(6, i, i % 2 === 0);
		set(i, 6, i % 2 === 0);
	}
	for (const [cx, cy] of [
		[3, 3],
		[SIZE - 4, 3],
		[3, SIZE - 4]
	]) {
		for (let dy = -4; dy <= 4; dy++) {
			for (let dx = -4; dx <= 4; dx++) {
				const x = cx + dx;
				const y = cy + dy;
				if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) continue;
				const d = Math.max(Math.abs(dx), Math.abs(dy));
				set(x, y, d !== 2 && d !== 4);
			}
		}
	}
	for (let dy = -2; dy <= 2; dy++) {
		for (let dx = -2; dx <= 2; dx++) {
			set(18 + dx, 18 + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1);
		}
	}

	// Format info: ECC L (01), mask 0 (000).
	const fmtData = 0b01000;
	let rem = fmtData;
	for (let i = 0; i < 10; i++) rem = (rem << 1) ^ ((rem >>> 9) * 0x537);
	const fmt = ((fmtData << 10) | rem) ^ 0x5412;
	const bit = (i: number) => ((fmt >>> i) & 1) !== 0;
	for (let i = 0; i <= 5; i++) set(8, i, bit(i));
	set(8, 7, bit(6));
	set(8, 8, bit(7));
	set(7, 8, bit(8));
	for (let i = 9; i < 15; i++) set(14 - i, 8, bit(i));
	for (let i = 0; i < 8; i++) set(SIZE - 1 - i, 8, bit(i));
	for (let i = 8; i < 15; i++) set(8, SIZE - 15 + i, bit(i));
	set(8, SIZE - 8, true);

	let k = 0;
	for (let right = SIZE - 1; right >= 1; right -= 2) {
		if (right === 6) right = 5;
		for (let vert = 0; vert < SIZE; vert++) {
			for (let j = 0; j < 2; j++) {
				const x = right - j;
				const upward = ((right + 1) & 2) === 0;
				const y = upward ? SIZE - 1 - vert : vert;
				if (fn[y][x]) continue;
				const dark = k < all.length * 8 && ((all[k >>> 3] >>> (7 - (k & 7))) & 1) === 1;
				m[y][x] = dark !== ((x + y) % 2 === 0);
				k++;
			}
		}
	}
	return m;
}

export function qrSvgPath(text: string): { size: number; d: string } {
	const m = qrMatrix(text);
	let d = '';
	m.forEach((row, y) =>
		row.forEach((dark, x) => {
			if (dark) d += `M${x + 4} ${y + 4}h1v1h-1z`;
		})
	);
	return { size: SIZE + 8, d };
}
