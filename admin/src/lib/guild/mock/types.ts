export type GuildRole = 'admin' | 'member' | 'absent' | 'no-seed';

export interface GuildActor {
  id: string;
}

export interface GuildMember {
  role: 'admin' | 'member';
  actor: string;
}

export interface GuildNode {
  name: string;
  nodeId: string;
  accessId: string;
}

export interface GuildOp {
  n: number;
  kind: string;
  id: string;
}

export interface GuildRecord {
  name: string;
  profile: 'house' | 'access-node';
  workplace: boolean;
  you: GuildRole;
  id: string;
  hex: string;
  outer: string | null;
  roster: GuildMember[];
  nodes: GuildNode[];
  ops: GuildOp[];
}
