/** Coarse bucket for a kernel interface name. */
export type IfaceKind = 'ethernet' | 'wifi' | 'vpn' | 'bridge' | 'virtual' | 'other';

export const KIND_ORDER: IfaceKind[] = ['ethernet', 'wifi', 'vpn', 'bridge', 'virtual', 'other'];

const KIND_LABEL: Record<IfaceKind, string> = {
	ethernet: 'Ethernet',
	wifi: 'Wi-Fi',
	vpn: 'VPN',
	bridge: 'Bridges',
	virtual: 'Virtual',
	other: 'Other'
};

export function kindLabel(kind: IfaceKind): string {
	return KIND_LABEL[kind];
}

export function classifyIface(name: string): IfaceKind {
	const n = name.toLowerCase();
	if (/^(tailscale|wg|tun|tap|zt|wireguard)/.test(n)) return 'vpn';
	if (/^(br[-.]|docker0$|virbr|virbr\d)/.test(n) || n === 'docker0') return 'bridge';
	if (/^(veth|docker|cni|flannel|cali|dummy|ifb|bond|sit|ip6|vxlan|geneve|nm-|vnet)/.test(n))
		return 'virtual';
	if (/^(wl|wlan)/.test(n)) return 'wifi';
	if (/^(eth|en|usb|lan)/.test(n)) return 'ethernet';
	return 'other';
}

/** Short name a person can scan. The kernel name stays available as detail. */
export function ifaceTitle(name: string): string {
	const n = name.toLowerCase();
	if (n.startsWith('tailscale')) return 'Tailscale';
	if (n.startsWith('wg') || n.startsWith('wireguard')) return 'WireGuard';
	if (n.startsWith('tun') || n.startsWith('tap')) return 'Tunnel';
	if (n.startsWith('zt')) return 'ZeroTier';
	const kind = classifyIface(name);
	if (kind === 'bridge') return 'Bridge';
	if (kind === 'ethernet') return 'Ethernet';
	if (kind === 'wifi') return 'Wi-Fi';
	if (kind === 'virtual') return 'Virtual';
	return name;
}
