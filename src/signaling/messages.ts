import { ClientMessage, ServerMessage } from "../types";

const ROOM_ID_RE = /^[a-zA-Z0-9_-]+$/;
const MAX_ROOM_ID_LEN = 64;
const MAX_DISPLAY_NAME_LEN = 128;
const MAX_SDP_LEN = 16384;
const MAX_CANDIDATE_LEN = 1024;
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export function parseClientMessage(raw: unknown): ClientMessage | null {
  let obj: unknown = raw;

  if (typeof raw === "string") {
    try {
      obj = JSON.parse(raw);
    } catch {
      return null;
    }
  }

  if (obj === null || typeof obj !== "object") {
    return null;
  }

  const record = obj as Record<string, unknown>;

  if (typeof record.type !== "string") {
    return null;
  }

  switch (record.type) {
    case "join-room": {
      if (typeof record.roomId !== "string") return null;
      if (record.roomId.length === 0 || record.roomId.length > MAX_ROOM_ID_LEN) return null;
      if (!ROOM_ID_RE.test(record.roomId)) return null;
      const msg: ClientMessage = { type: "join-room", roomId: record.roomId };
      if (typeof record.displayName === "string") {
        if (record.displayName.length > MAX_DISPLAY_NAME_LEN) return null;
        msg.displayName = record.displayName;
      }
      return msg;
    }

    case "leave-room":
      return { type: "leave-room" };

    case "offer":
      if (typeof record.targetPeerId !== "string" || typeof record.sdp !== "string") {
        return null;
      }
      if (!UUID_RE.test(record.targetPeerId)) return null;
      if (record.sdp.length > MAX_SDP_LEN) return null;
      return { type: "offer", targetPeerId: record.targetPeerId, sdp: record.sdp };

    case "answer":
      if (typeof record.targetPeerId !== "string" || typeof record.sdp !== "string") {
        return null;
      }
      if (!UUID_RE.test(record.targetPeerId)) return null;
      if (record.sdp.length > MAX_SDP_LEN) return null;
      return { type: "answer", targetPeerId: record.targetPeerId, sdp: record.sdp };

    case "ice-candidate":
      if (typeof record.targetPeerId !== "string" || typeof record.candidate !== "string") {
        return null;
      }
      if (!UUID_RE.test(record.targetPeerId)) return null;
      if (record.candidate.length > MAX_CANDIDATE_LEN) return null;
      return { type: "ice-candidate", targetPeerId: record.targetPeerId, candidate: record.candidate };

    case "ping":
      return { type: "ping" };

    default:
      return null;
  }
}

export function serializeServerMessage(msg: ServerMessage): string {
  return JSON.stringify(msg);
}
