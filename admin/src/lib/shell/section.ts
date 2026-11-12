export type SectionId = 'home' | 'backhaul' | 'guild' | 'map' | 'network' | 'services' | 'you';

/** Which section bar row is current, including pages that are not sections. */
export function sectionFor(pathname: string): SectionId {
	if (pathname === '/backhaul' || pathname.startsWith('/backhaul/')) return 'backhaul';
	if (
		pathname === '/guild' ||
		pathname.startsWith('/guild/') ||
		pathname === '/group' ||
		pathname.startsWith('/group/')
	) {
		return 'guild';
	}
	if (pathname === '/map' || pathname.startsWith('/map/')) return 'map';
	if (
		pathname === '/network' ||
		pathname.startsWith('/network/') ||
		pathname === '/nearby' ||
		pathname.startsWith('/nearby/')
	) {
		return 'network';
	}
	if (pathname === '/services' || pathname.startsWith('/services/')) return 'services';
	if (
		pathname === '/you' ||
		pathname.startsWith('/you/') ||
		pathname === '/become' ||
		pathname.startsWith('/become/')
	) {
		return 'you';
	}
	return 'home';
}
