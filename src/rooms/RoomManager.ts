import { PeerId, RoomId, PeerInfo } from "../types";
import { Room } from "./Room";

export class RoomManager {
  private rooms: Map<RoomId, Room>;
  private peerRooms: Map<PeerId, RoomId>; // Reverse lookup

  constructor() {
    this.rooms = new Map();
    this.peerRooms = new Map();
  }

  // Room lifecycle - rooms are lazy-created on first join, auto-destroyed when empty
  joinRoom(
    peerId: PeerId,
    roomId: RoomId,
    displayName: string
  ): {
    room: Room;
    isNew: boolean;
    previousRoomId: RoomId | null;
    previousRoomDeleted: boolean;
  } {
    const currentRoomId = this.peerRooms.get(peerId) ?? null;

    if (currentRoomId === roomId) {
      const error: Error & { code?: string } = new Error(
        `Peer ${peerId} is already in room ${roomId}`
      );
      error.code = "ALREADY_IN_ROOM";
      throw error;
    }

    // Implicit leave from previous room
    let previousRoomDeleted = false;
    if (currentRoomId !== null) {
      const previousRoom = this.rooms.get(currentRoomId);
      if (previousRoom) {
        previousRoom.removePeer(peerId);
        if (previousRoom.isEmpty) {
          this.rooms.delete(currentRoomId);
          previousRoomDeleted = true;
        }
      }
      this.peerRooms.delete(peerId);
    }

    // Lazy-create the target room if it doesn't exist
    let room = this.rooms.get(roomId);
    const isNew = room === undefined;
    if (!room) {
      room = new Room(roomId);
      this.rooms.set(roomId, room);
    }

    const peerInfo: PeerInfo = {
      peerId,
      roomId,
      displayName,
      joinedAt: Date.now(),
    };

    room.addPeer(peerInfo); // Throws if full
    this.peerRooms.set(peerId, roomId);

    return {
      room,
      isNew,
      previousRoomId: currentRoomId,
      previousRoomDeleted,
    };
  }

  leaveRoom(peerId: PeerId): { roomId: RoomId; roomDeleted: boolean } | null {
    const roomId = this.peerRooms.get(peerId);
    if (roomId === undefined) {
      return null;
    }

    const room = this.rooms.get(roomId);
    if (room) {
      room.removePeer(peerId);
      if (room.isEmpty) {
        this.rooms.delete(roomId);
        this.peerRooms.delete(peerId);
        return { roomId, roomDeleted: true };
      }
    }

    this.peerRooms.delete(peerId);
    return { roomId, roomDeleted: false };
  }

  getPeerRoom(peerId: PeerId): RoomId | null {
    return this.peerRooms.get(peerId) ?? null;
  }

  arePeersInSameRoom(peerId1: PeerId, peerId2: PeerId): boolean {
    const room1 = this.peerRooms.get(peerId1);
    const room2 = this.peerRooms.get(peerId2);
    return room1 !== undefined && room1 === room2;
  }

  // Cleanup on disconnect - same semantics as leaveRoom
  removePeer(peerId: PeerId): { roomId: RoomId; roomDeleted: boolean } | null {
    return this.leaveRoom(peerId);
  }

  getRoomCount(): number {
    return this.rooms.size;
  }

  getPeerCount(): number {
    return this.peerRooms.size;
  }

  getRoomInfo(roomId: RoomId): Room | null {
    return this.rooms.get(roomId) ?? null;
  }

  getAllRooms(): RoomId[] {
    return Array.from(this.rooms.keys());
  }
}
