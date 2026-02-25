import { describe, test, expect, beforeEach } from "bun:test";
import { RoomManager } from "../rooms/RoomManager";
import { Room } from "../rooms/Room";

describe("RoomManager", () => {
  let rm: RoomManager;

  beforeEach(() => {
    rm = new RoomManager();
  });

  describe("joinRoom - room auto-creation", () => {
    test("joining a non-existent room creates it and returns isNew: true", () => {
      const result = rm.joinRoom("peer-1", "room-a", "Alice");
      expect(result.isNew).toBe(true);
      expect(result.room).toBeInstanceOf(Room);
      expect(result.room.roomId).toBe("room-a");
      expect(result.previousRoomId).toBeNull();
      expect(result.previousRoomDeleted).toBe(false);
    });

    test("second peer joining existing room gets isNew: false", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const result = rm.joinRoom("peer-2", "room-a", "Bob");
      expect(result.isNew).toBe(false);
      expect(result.room.roomId).toBe("room-a");
    });
  });

  describe("joinRoom - peer list", () => {
    test("room-joined peers list contains existing peers when second peer joins", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const result = rm.joinRoom("peer-2", "room-a", "Bob");
      const peerList = result.room.getPeerList();
      const peerIds = peerList.map((p) => p.peerId);
      expect(peerIds).toContain("peer-1");
      expect(peerIds).toContain("peer-2");
      expect(peerList).toHaveLength(2);
    });
  });

  describe("leaveRoom - auto-delete empty room", () => {
    test("leaving the last peer deletes the room and returns roomDeleted: true", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const result = rm.leaveRoom("peer-1");
      expect(result).not.toBeNull();
      expect(result!.roomId).toBe("room-a");
      expect(result!.roomDeleted).toBe(true);
      expect(rm.getRoomCount()).toBe(0);
    });

    test("leaving when other peers remain returns roomDeleted: false", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.joinRoom("peer-2", "room-a", "Bob");
      const result = rm.leaveRoom("peer-1");
      expect(result).not.toBeNull();
      expect(result!.roomDeleted).toBe(false);
      expect(rm.getRoomCount()).toBe(1);
    });
  });

  describe("joinRoom - capacity limit", () => {
    test("third peer joining a maxPeers=2 room throws", () => {
      // RoomManager creates rooms with default maxPeers=10; we create a Room directly
      // and inject scenario by filling via the manager.
      // Since RoomManager always uses new Room(roomId) with default 10, we bypass by
      // adding 10 peers and testing the 11th.
      for (let i = 1; i <= 10; i++) {
        rm.joinRoom(`peer-${i}`, "room-full", `Peer${i}`);
      }
      expect(() => rm.joinRoom("peer-11", "room-full", "Overflow")).toThrow();
    });

    test("exceeding capacity throws an error (not ALREADY_IN_ROOM)", () => {
      for (let i = 1; i <= 10; i++) {
        rm.joinRoom(`peer-${i}`, "room-full", `Peer${i}`);
      }
      let thrownError: Error | null = null;
      try {
        rm.joinRoom("peer-11", "room-full", "Overflow");
      } catch (err) {
        thrownError = err as Error;
      }
      expect(thrownError).not.toBeNull();
      expect((thrownError as Error & { code?: string }).code).not.toBe("ALREADY_IN_ROOM");
    });
  });

  describe("joinRoom - already in room", () => {
    test("joining the same room twice throws with code ALREADY_IN_ROOM", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      let thrownError: Error & { code?: string } | null = null;
      try {
        rm.joinRoom("peer-1", "room-a", "Alice");
      } catch (err) {
        thrownError = err as Error & { code?: string };
      }
      expect(thrownError).not.toBeNull();
      expect(thrownError!.code).toBe("ALREADY_IN_ROOM");
    });
  });

  describe("joinRoom - implicit leave", () => {
    test("joining a new room while in another returns previousRoomId", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const result = rm.joinRoom("peer-1", "room-b", "Alice");
      expect(result.previousRoomId).toBe("room-a");
    });

    test("implicit leave: peer is removed from old room", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.joinRoom("peer-1", "room-b", "Alice");
      const oldRoom = rm.getRoomInfo("room-a");
      // room-a had only peer-1, so it should be deleted after implicit leave
      expect(oldRoom).toBeNull();
    });

    test("implicit leave deletes old room if peer was last - previousRoomDeleted: true", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const result = rm.joinRoom("peer-1", "room-b", "Alice");
      expect(result.previousRoomDeleted).toBe(true);
    });

    test("implicit leave does not delete old room if others remain - previousRoomDeleted: false", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.joinRoom("peer-2", "room-a", "Bob");
      const result = rm.joinRoom("peer-1", "room-b", "Alice");
      expect(result.previousRoomDeleted).toBe(false);
      expect(rm.getRoomInfo("room-a")).not.toBeNull();
    });
  });

  describe("getPeerRoom", () => {
    test("returns correct roomId for a peer in a room", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      expect(rm.getPeerRoom("peer-1")).toBe("room-a");
    });

    test("returns null for a peer not in any room", () => {
      expect(rm.getPeerRoom("ghost")).toBeNull();
    });

    test("returns null after peer leaves", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.leaveRoom("peer-1");
      expect(rm.getPeerRoom("peer-1")).toBeNull();
    });
  });

  describe("arePeersInSameRoom", () => {
    test("returns true when both peers are in the same room", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.joinRoom("peer-2", "room-a", "Bob");
      expect(rm.arePeersInSameRoom("peer-1", "peer-2")).toBe(true);
    });

    test("returns false when peers are in different rooms", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.joinRoom("peer-2", "room-b", "Bob");
      expect(rm.arePeersInSameRoom("peer-1", "peer-2")).toBe(false);
    });

    test("returns false when one peer is not in any room", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      expect(rm.arePeersInSameRoom("peer-1", "ghost")).toBe(false);
    });

    test("returns false when neither peer is in any room", () => {
      expect(rm.arePeersInSameRoom("ghost-1", "ghost-2")).toBe(false);
    });
  });

  describe("removePeer", () => {
    test("removePeer has same semantics as leaveRoom", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const result = rm.removePeer("peer-1");
      expect(result).not.toBeNull();
      expect(result!.roomId).toBe("room-a");
      expect(result!.roomDeleted).toBe(true);
    });

    test("removePeer on peer not in any room returns null", () => {
      expect(rm.removePeer("ghost")).toBeNull();
    });
  });

  describe("introspection", () => {
    test("getRoomCount returns correct count", () => {
      expect(rm.getRoomCount()).toBe(0);
      rm.joinRoom("peer-1", "room-a", "Alice");
      expect(rm.getRoomCount()).toBe(1);
      rm.joinRoom("peer-2", "room-b", "Bob");
      expect(rm.getRoomCount()).toBe(2);
      rm.leaveRoom("peer-1");
      expect(rm.getRoomCount()).toBe(1);
    });

    test("getPeerCount returns correct count", () => {
      expect(rm.getPeerCount()).toBe(0);
      rm.joinRoom("peer-1", "room-a", "Alice");
      expect(rm.getPeerCount()).toBe(1);
      rm.joinRoom("peer-2", "room-a", "Bob");
      expect(rm.getPeerCount()).toBe(2);
      rm.leaveRoom("peer-1");
      expect(rm.getPeerCount()).toBe(1);
    });

    test("getAllRooms returns all active room IDs", () => {
      expect(rm.getAllRooms()).toHaveLength(0);
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.joinRoom("peer-2", "room-b", "Bob");
      const rooms = rm.getAllRooms();
      expect(rooms).toHaveLength(2);
      expect(rooms).toContain("room-a");
      expect(rooms).toContain("room-b");
    });

    test("getAllRooms does not include deleted rooms", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      rm.leaveRoom("peer-1");
      expect(rm.getAllRooms()).toHaveLength(0);
    });

    test("getRoomInfo returns Room instance for existing room", () => {
      rm.joinRoom("peer-1", "room-a", "Alice");
      const room = rm.getRoomInfo("room-a");
      expect(room).toBeInstanceOf(Room);
      expect(room!.roomId).toBe("room-a");
    });

    test("getRoomInfo returns null for non-existent room", () => {
      expect(rm.getRoomInfo("no-such-room")).toBeNull();
    });
  });

  describe("leaveRoom - not in room", () => {
    test("leaveRoom on peer not in any room returns null", () => {
      expect(rm.leaveRoom("ghost")).toBeNull();
    });
  });
});
