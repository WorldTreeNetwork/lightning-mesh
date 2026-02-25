import { PeerId, ClientMessage, SignalingAction, ErrorCode, ServerMessage } from "../types";
import { RoomManager } from "../rooms/RoomManager";

function sendError(targetPeerId: PeerId, code: ErrorCode, message: string): SignalingAction {
  return {
    type: "send",
    targetPeerId,
    message: { type: "error", code, message },
  };
}

export function handleMessage(
  peerId: PeerId,
  message: ClientMessage,
  roomManager: RoomManager
): SignalingAction[] {
  switch (message.type) {
    case "join-room": {
      const actions: SignalingAction[] = [];

      let result: ReturnType<RoomManager["joinRoom"]>;
      try {
        result = roomManager.joinRoom(peerId, message.roomId, message.displayName ?? "");
      } catch (err) {
        const code = (err as { code?: string }).code === "ALREADY_IN_ROOM"
          ? "ALREADY_IN_ROOM"
          : "ROOM_FULL";
        const errMsg = err instanceof Error ? err.message : String(err);
        return [sendError(peerId, code as ErrorCode, errMsg)];
      }

      const { room, previousRoomId } = result;

      // If peer was previously in another room, publish peer-left to that room
      if (previousRoomId !== null) {
        actions.push({
          type: "publish",
          roomId: previousRoomId,
          message: { type: "peer-left", peerId },
        });
      }

      // Send room-joined to the joining peer (list of OTHER peers already in room)
      const otherPeers = room.getPeerList().filter((p) => p.peerId !== peerId);
      actions.push({
        type: "send",
        targetPeerId: peerId,
        message: { type: "room-joined", roomId: message.roomId, peers: otherPeers },
      });

      // Publish peer-joined to room, excluding the joining peer
      const joinerInfo = room.peers.get(peerId);
      const displayName = joinerInfo?.displayName ?? message.displayName ?? "";
      actions.push({
        type: "publish",
        roomId: message.roomId,
        message: { type: "peer-joined", peerId, displayName },
        excludePeerId: peerId,
      });

      return actions;
    }

    case "leave-room": {
      const result = roomManager.leaveRoom(peerId);
      if (result === null) {
        return [sendError(peerId, "NOT_IN_ROOM", "You are not currently in a room.")];
      }
      return [
        {
          type: "publish",
          roomId: result.roomId,
          message: { type: "peer-left", peerId },
        },
      ];
    }

    case "offer":
    case "answer":
    case "ice-candidate": {
      const senderRoom = roomManager.getPeerRoom(peerId);
      if (senderRoom === null) {
        return [sendError(peerId, "NOT_IN_ROOM", "You are not currently in a room.")];
      }

      if (message.targetPeerId === peerId) {
        return [sendError(peerId, "PEER_NOT_FOUND", "Cannot send to yourself.")];
      }

      if (!roomManager.arePeersInSameRoom(peerId, message.targetPeerId)) {
        return [sendError(peerId, "PEER_NOT_FOUND", "Target peer not found in your room.")];
      }

      let outgoing: ServerMessage;
      if (message.type === "offer") {
        outgoing = { type: "offer", fromPeerId: peerId, sdp: message.sdp };
      } else if (message.type === "answer") {
        outgoing = { type: "answer", fromPeerId: peerId, sdp: message.sdp };
      } else {
        outgoing = { type: "ice-candidate", fromPeerId: peerId, candidate: message.candidate };
      }

      return [
        {
          type: "send",
          targetPeerId: message.targetPeerId,
          message: outgoing,
        },
      ];
    }

    case "ping":
      return [
        {
          type: "send",
          targetPeerId: peerId,
          message: { type: "pong" },
        },
      ];
  }
}
