import { describe, expect, it } from 'vitest';
import type { GuildSnapshot } from '$lib/guild/api';
import {
	countGroup,
	formatCount,
	groupsForHome,
	groupsFromSnapshot,
	heardRouters,
	houseTiles,
	LOCAL,
	localTiles,
	needsAdmin,
	present,
	routerTitle,
	selectionCount
} from './groups';

const heard = [
	{ id: 'aa', name: 'ap3000' },
	{ id: 'bb', name: 'tr3000' },
	{ id: 'cc', name: 'mt3000' }
];

describe('group counts', () => {
	it('counts heard routers over the assigned roster', () => {
		const count = countGroup(
			{
				id: 'house',
				name: 'House',
				assigned: [
					{ id: 'AA', name: 'ap3000' },
					{ id: 'bb', name: 'tr3000' },
					{ id: 'dd', name: 'spare' },
					{ id: 'ee', name: 'loft' },
					{ id: 'ff', name: 'hall' }
				]
			},
			heard
		);
		expect(formatCount(count)).toBe('2/5');
		expect(count.heard.map((router) => router.name)).toEqual(['ap3000', 'tr3000']);
	});

	it('uses a placeholder when the group has no roster', () => {
		const count = countGroup({ id: 'scan', name: 'This scan', assigned: [] }, heard);
		expect(formatCount(count)).toBe('3/–');
		expect(count.heard).toHaveLength(3);
	});

	it('drops failed announcements and this computer', () => {
		expect(
			heardRouters([
				{ meshName: 'ap3000', meshNodeId: 'aa', local: false, state: 'reachable' },
				{ meshName: 'ap3000', meshNodeId: 'aa', local: false, state: 'reachable' },
				{ meshName: 'gone', meshNodeId: 'zz', local: false, state: 'failed' },
				{ meshName: 'laptop', meshNodeId: 'me', local: true, state: 'local' }
			]).map((router) => router.id)
		).toEqual(['aa']);
	});

	it('lists house guilds and skips access-node guilds', () => {
		const snapshot = {
			guilds: [
				{ name: 'door', profile: 'access-node', workplace: false, id: 'a', keyspaceId: 'a', nodes: [] },
				{ name: 'House', profile: 'house', workplace: false, id: 'h', keyspaceId: 'h', nodes: [{ nodeId: 'aa', name: 'ap3000' }] },
				{ name: 'Shop', profile: 'house', workplace: true, id: 's', keyspaceId: 's', nodes: [] }
			]
		} as unknown as GuildSnapshot;
		expect(groupsFromSnapshot(snapshot).map((group) => group.name)).toEqual(['Shop', 'House']);
	});

	it('asks for admin when this person is not one', () => {
		expect(needsAdmin('member')).toBe(true);
		expect(needsAdmin('absent')).toBe(true);
		expect(needsAdmin('no-seed')).toBe(true);
		expect(needsAdmin('admin')).toBe(false);
		expect(needsAdmin(undefined)).toBe(false);
	});

	it('keeps missed routers on the house, in roster order', () => {
		const tiles = houseTiles(
			{
				id: 'house',
				name: 'House',
				assigned: [
					{ id: 'AA', name: 'ap3000' },
					{ id: 'dd', name: 'spare' }
				]
			},
			heard
		);
		expect(tiles).toEqual([
			{ id: 'AA', name: 'ap3000', heard: true },
			{ id: 'dd', name: 'spare', heard: false }
		]);
	});

	it('shows the scan when the group has no roster', () => {
		expect(houseTiles({ id: 'scan', name: 'This scan', assigned: [] }, heard)).toEqual([
			{ id: 'aa', name: 'ap3000', heard: true },
			{ id: 'bb', name: 'tr3000', heard: true },
			{ id: 'cc', name: 'mt3000', heard: true }
		]);
	});

	it('falls back to Local and counts the nodes this computer can see', () => {
		const view = present([], '', heard, [
			{ id: 'aa', name: 'ap3000' },
			{ id: 'gw', name: 'Gateway' }
		]);
		expect(view.selected).toEqual(LOCAL);
		expect(view.list.map((row) => row.group.name)).toEqual(['Local']);
		expect(selectionCount(view.selected, view.count)).toBe('2');
	});

	it('keeps a workplace and drops the Admin guild from Home', () => {
		const groups = groupsForHome([
			{ id: 'admin', name: 'Admin', assigned: [], workplace: false },
			{ id: 'house', name: 'Strawberry Creek', assigned: [{ id: 'aa', name: 'ap3000' }], workplace: true }
		]);
		const view = present(groups, 'house', heard, [{ id: 'aa', name: 'ap3000' }]);
		expect(view.list.map((row) => row.group.name)).toEqual(['Local', 'Strawberry Creek']);
		expect(view.selected.name).toBe('Strawberry Creek');
		expect(selectionCount(view.selected, view.count)).toBe('1/1');
	});

	it('lists seen hosts and opens a lightning node', () => {
		const tiles = localTiles([
			{
				id: 'me',
				local: true,
				state: 'local',
				meshName: null,
				meshNodeId: null,
				hostname: null,
				router: false,
				primary: { address: '10.0.0.2' }
			},
			{
				id: 'dead',
				local: false,
				state: 'failed',
				meshName: null,
				meshNodeId: null,
				hostname: 'gone.local',
				router: false,
				primary: { address: '10.0.0.9' }
			},
			{
				id: 'ap',
				local: false,
				state: 'reachable',
				meshName: 'ap3000',
				meshNodeId: 'aa',
				hostname: null,
				router: true,
				primary: { address: '10.42.1.1' }
			},
			{
				id: 'ssdp',
				local: false,
				state: 'reachable',
				meshName: null,
				meshNodeId: null,
				hostname: null,
				router: false,
				primary: { address: '239.255.255.250' }
			},
			{
				id: 'phone',
				local: false,
				state: 'reachable',
				meshName: null,
				meshNodeId: null,
				hostname: 'pixel.local',
				router: false,
				primary: { address: '192.168.1.20' }
			}
		]);
		expect(tiles).toEqual([
			{ id: 'aa', name: 'ap3000', detail: '10.42.1.1', href: '/router/aa' },
			{ id: 'phone', name: 'pixel', detail: '192.168.1.20', href: '/host/phone' }
		]);
	});

	it('titles a known router by its name', () => {
		const group = {
			id: 'house',
			name: 'House',
			assigned: [{ id: 'AA', name: 'ap3000' }]
		};
		const named = [
			{ id: 'AA', name: 'other' },
			{ id: 'bb', name: 'tr3000' }
		];
		expect(routerTitle('AA', group, named)).toBe('ap3000');
		expect(routerTitle('bb', group, named)).toBe('tr3000');
		expect(routerTitle('zz', group, named)).toBe('zz');
		expect(routerTitle('  ', group, named)).toBe('Router');
	});
});
