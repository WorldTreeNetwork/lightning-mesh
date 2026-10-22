import { describe, expect, test } from 'vitest';
import { classifyIface, ifaceTitle } from './groups';

describe('interface groups', () => {
	test('puts tunnels and overlays in vpn', () => {
		expect(classifyIface('tailscale0')).toBe('vpn');
		expect(classifyIface('wg0')).toBe('vpn');
		expect(ifaceTitle('tailscale0')).toBe('Tailscale');
		expect(ifaceTitle('wg0')).toBe('WireGuard');
	});

	test('collapses docker bridges to a type, not the hex name', () => {
		expect(classifyIface('br-4fbd877829e3')).toBe('bridge');
		expect(ifaceTitle('br-4fbd877829e3')).toBe('Bridge');
		expect(classifyIface('docker0')).toBe('bridge');
		expect(classifyIface('veth1a2b')).toBe('virtual');
	});

	test('keeps physical links distinct', () => {
		expect(classifyIface('enp0s31f6')).toBe('ethernet');
		expect(classifyIface('wlp191s0')).toBe('wifi');
		expect(ifaceTitle('enp0s31f6')).toBe('Ethernet');
	});
});
