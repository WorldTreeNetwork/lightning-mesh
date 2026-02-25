// Unique identifier for a peer within a session
export type PeerId = string; // UUID v4, generated server-side on connect
export type RoomId = string; // Alphanumeric slug, provided by the client

export interface PeerInfo {
  peerId: PeerId;
  roomId: RoomId | null;
  displayName: string;
  joinedAt: number; // Unix timestamp ms
}

export interface PeerSummary {
  peerId: PeerId;
  displayName: string;
}

// --- Client -> Server Messages ---

export type ClientMessage =
  | { type: "join-room"; roomId: RoomId; displayName?: string }
  | { type: "leave-room" }
  | { type: "offer"; targetPeerId: PeerId; sdp: string }
  | { type: "answer"; targetPeerId: PeerId; sdp: string }
  | { type: "ice-candidate"; targetPeerId: PeerId; candidate: string }
  | { type: "ping" };

// --- Server -> Client Messages ---

export type ServerMessage =
  | { type: "welcome"; peerId: PeerId }
  | { type: "room-joined"; roomId: RoomId; peers: PeerSummary[] }
  | { type: "peer-joined"; peerId: PeerId; displayName: string }
  | { type: "peer-left"; peerId: PeerId }
  | { type: "offer"; fromPeerId: PeerId; sdp: string }
  | { type: "answer"; fromPeerId: PeerId; sdp: string }
  | { type: "ice-candidate"; fromPeerId: PeerId; candidate: string }
  | { type: "error"; code: ErrorCode; message: string }
  | { type: "pong" };

export type ErrorCode =
  | "ROOM_FULL"
  | "ROOM_NOT_FOUND"
  | "ALREADY_IN_ROOM"
  | "NOT_IN_ROOM"
  | "PEER_NOT_FOUND"
  | "INVALID_MESSAGE"
  | "RATE_LIMITED";

// --- Signaling Actions (returned by handlers, executed by WS layer) ---

export interface SendAction {
  type: "send";
  targetPeerId: PeerId;
  message: ServerMessage;
}

export interface PublishAction {
  type: "publish";
  roomId: RoomId;
  message: ServerMessage;
  excludePeerId?: PeerId;
}

export type SignalingAction = SendAction | PublishAction;
