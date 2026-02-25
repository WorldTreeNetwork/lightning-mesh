import { PeerId, RoomId, PeerInfo, PeerSummary } from "../types";

export class Room {
  readonly roomId: RoomId;
  readonly peers: Map<PeerId, PeerInfo>;
  readonly createdAt: number;
  readonly maxPeers: number;

  constructor(roomId: RoomId, maxPeers: number = 10) {
    this.roomId = roomId;
    this.peers = new Map();
    this.createdAt = Date.now();
    this.maxPeers = maxPeers;
  }

  addPeer(peer: PeerInfo): void {
    if (this.isFull) {
      throw new Error(`Room ${this.roomId} is full (max ${this.maxPeers} peers)`);
    }
    this.peers.set(peer.peerId, peer);
  }

  removePeer(peerId: PeerId): void {
    this.peers.delete(peerId);
  }

  hasPeer(peerId: PeerId): boolean {
    return this.peers.has(peerId);
  }

  getPeerList(): PeerSummary[] {
    return Array.from(this.peers.values()).map((peer) => ({
      peerId: peer.peerId,
      displayName: peer.displayName,
    }));
  }

  get size(): number {
    return this.peers.size;
  }

  get isFull(): boolean {
    return this.peers.size >= this.maxPeers;
  }

  get isEmpty(): boolean {
    return this.peers.size === 0;
  }
}
