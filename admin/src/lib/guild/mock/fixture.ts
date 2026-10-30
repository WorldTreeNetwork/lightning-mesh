import type { GuildActor, GuildRecord } from './types';

const ACTOR_ID = '9KsF3Ce5qh1vsFCc6GDaSCPpNJ56YR4nnyLffKuLpy42';
const ADMIN_GUILD_ID = '4qrcusxzKCdietwYgAU8NBs3VNVBnmP95eV3jynSbttz';
const STRAWBERRY_CREEK_ID = 'GdWfE9wt5ERKhXGbDr1w6RZS38JFX77aqeHX8sFvsZGa';
const ACCESS_NODE_ID = 'GmsF7ufeeu15D5tgVLNS6ExAhEprLgq15LpGhQvga5C9';

export const mockActor: GuildActor = {
  id: ACTOR_ID,
};

export const mockGuilds: GuildRecord[] = [
  {
    name: 'Admin',
    profile: 'house',
    workplace: false,
    you: 'admin',
    id: ADMIN_GUILD_ID,
    hex: '39170b4fc5694c218c2eb163285a429900144119f6732b2d289b6dc1f2a13bf3',
    outer: null,
    roster: [{ role: 'admin', actor: ACTOR_ID }],
    nodes: [],
    ops: [{ n: 1, kind: 'keyspace.join', id: ADMIN_GUILD_ID }],
  },
  {
    name: 'Strawberry Creek',
    profile: 'house',
    workplace: true,
    you: 'admin',
    id: STRAWBERRY_CREEK_ID,
    hex: 'e839bfa7c53ac371ca66b6b973dcaa3fcf296a0617258423ba8d84d8f91b6ae7',
    outer: null,
    roster: [{ role: 'admin', actor: ACTOR_ID }],
    nodes: [
      {
        name: 'ap3000-outdoor-b',
        nodeId: 'a7a8841b13de5fbf20daec51318bad40dffc4425853edda6e9defc59af374037',
        accessId: ACCESS_NODE_ID,
      },
    ],
    ops: [
      { n: 2, kind: 'keyspace.join', id: STRAWBERRY_CREEK_ID },
      { n: 3, kind: 'keyspace.assign-node', id: '2Nwkvi57u4dpVPRcLnvRzxhyWWmAmUdHtt1191RR8gFc' },
      { n: 4, kind: 'keyspace.nest', id: ACCESS_NODE_ID },
    ],
  },
  {
    name: 'access-node',
    profile: 'access-node',
    workplace: false,
    you: 'admin',
    id: ACCESS_NODE_ID,
    hex: 'ea5dab8037d659444a3291c4e818091f57522bd59adcc36bfbf352ce95fae28e',
    outer: STRAWBERRY_CREEK_ID,
    roster: [{ role: 'admin', actor: ACTOR_ID }],
    nodes: [],
    ops: [{ n: 4, kind: 'keyspace.nest', id: ACCESS_NODE_ID }],
  },
];
