import { Elysia } from "elysia";
import { RoomManager } from "../rooms/RoomManager";
import { config } from "../config";

export function createHealthRoutes(roomManager: RoomManager) {
  return new Elysia()
    .get("/health", () => ({
      status: "ok",
      uptime: process.uptime(),
    }))
    .get("/rooms", () => ({
      rooms: roomManager.getAllRooms().map(roomId => {
        const room = roomManager.getRoomInfo(roomId);
        return {
          roomId,
          peerCount: room?.peers.size ?? 0,
        };
      }),
      totalRooms: roomManager.getRoomCount(),
      totalPeers: roomManager.getPeerCount(),
    }))
    .get("/rooms/:roomId", ({ params, set, headers }) => {
      const room = roomManager.getRoomInfo(params.roomId);
      if (!room) {
        set.status = 404;
        return { error: "Room not found" };
      }

      const isAdminTokenRequired = config.adminToken !== "";
      const isAuthenticated =
        !isAdminTokenRequired ||
        headers.authorization === `Bearer ${config.adminToken}`;

      const peers = [...room.peers.values()].map(p =>
        isAuthenticated
          ? { peerId: p.peerId, displayName: p.displayName, joinedAt: p.joinedAt }
          : { displayName: p.displayName, joinedAt: p.joinedAt }
      );

      return {
        roomId: room.roomId,
        peers,
        peerCount: room.size,
        createdAt: room.createdAt,
      };
    });
}
