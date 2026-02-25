import { Elysia } from "elysia";
import type { PeerId, SignalingAction } from "../types";
import { RoomManager } from "../rooms/RoomManager";
import { handleMessage } from "../signaling/handlers";
import { parseClientMessage, serializeServerMessage } from "../signaling/messages";
import { logger } from "../logger";
import { config } from "../config";

// Minimal interface for the subset of ElysiaWS we use in maps.
// Elysia's WS generic type is complex; this captures the methods we need.
interface WsHandle {
  readonly id: string;
  send(data: string | ArrayBufferLike): number;
  publish(topic: string, data: string): number;
  subscribe(topic: string): void;
  unsubscribe(topic: string): void;
}

function asWsHandle(ws: unknown): WsHandle {
  return ws as WsHandle;
}

// --- Rate Limiter (sliding window per connection) ---

interface RateBucket {
  timestamps: number[];
}

const rateLimits = new Map<string, RateBucket>();

function isRateLimited(socketId: string): boolean {
  const now = Date.now();
  const windowMs = 1000;
  let bucket = rateLimits.get(socketId);
  if (!bucket) {
    bucket = { timestamps: [] };
    rateLimits.set(socketId, bucket);
  }
  // Prune timestamps outside the window
  bucket.timestamps = bucket.timestamps.filter(t => now - t < windowMs);
  if (bucket.timestamps.length >= config.rateLimitPerSecond) {
    return true;
  }
  bucket.timestamps.push(now);
  return false;
}

// --- Factory ---

export function createSignalingWs(roomManager: RoomManager) {
  // Per-instance bidirectional mapping: peerId <-> socket
  const peerSockets = new Map<PeerId, WsHandle>();
  const socketIdToPeer = new Map<string, PeerId>();

  function executeActions(actions: SignalingAction[], senderWs: WsHandle): void {
    for (const action of actions) {
      switch (action.type) {
        case "send": {
          const targetSocket = peerSockets.get(action.targetPeerId);
          if (targetSocket) {
            const status = targetSocket.send(serializeServerMessage(action.message));
            if (status === 0) {
              logger.warn({ peerId: action.targetPeerId }, "Message dropped");
            }
          }
          break;
        }
        case "publish": {
          // Bun's ws.publish() natively excludes the sender socket (publishToSelf=false).
          // The excludePeerId field on PublishAction exists for documentation but is not
          // explicitly enforced here because Bun handles it at the C++ layer.
          const serialized = serializeServerMessage(action.message);
          senderWs.publish(action.roomId, serialized);
          break;
        }
      }
    }
  }

  return new Elysia()
    .ws("/ws", {
      body: undefined as unknown as string,
      maxPayloadLength: 64 * 1024, // 64KB - SDPs and ICE candidates are small
      idleTimeout: 300,            // 5 minutes - peers may keep signaling open after WebRTC establishes

      open(ws) {
        const handle = asWsHandle(ws);
        const peerId = crypto.randomUUID();
        peerSockets.set(peerId, handle);
        socketIdToPeer.set(handle.id, peerId);

        ws.send(serializeServerMessage({ type: "welcome", peerId }));
        logger.debug({ peerId }, "Peer connected");
      },

      message(ws, rawMessage) {
        const handle = asWsHandle(ws);
        const peerId = socketIdToPeer.get(handle.id);
        if (!peerId) return;

        // Rate limiting check (before parsing to minimize CPU for abusive clients)
        if (isRateLimited(handle.id)) {
          ws.send(serializeServerMessage({
            type: "error",
            code: "RATE_LIMITED",
            message: "Too many messages",
          }));
          logger.warn({ peerId }, "Rate limited");
          return;
        }

        const message = parseClientMessage(rawMessage);
        if (!message) {
          ws.send(serializeServerMessage({
            type: "error",
            code: "INVALID_MESSAGE",
            message: "Malformed or unrecognized message",
          }));
          return;
        }

        // ORDERING INVARIANT: Subscription updates MUST happen BEFORE executeActions().
        //
        // When a peer joins a room, the handler adds them to the room (state change),
        // then returns a PublishAction(peer-joined) to notify existing room members.
        // We must subscribe the joining peer to the new room topic before executing
        // the publish action, and unsubscribe from the old room if they switched rooms.
        //
        // For leave-room, we unsubscribe first, then execute the peer-left publish.
        // The leaving peer's socket can still call ws.publish() to a topic it's not
        // subscribed to -- Bun routes to all OTHER subscribers regardless.
        //
        // Sequence: 1) Capture old room  2) Run handler (mutates state)
        //           3) Capture new room  4) Update subscriptions  5) Execute actions
        const oldRoomId = roomManager.getPeerRoom(peerId);

        const actions = handleMessage(peerId, message, roomManager);

        const newRoomId = roomManager.getPeerRoom(peerId);
        if (oldRoomId !== newRoomId) {
          if (oldRoomId) ws.unsubscribe(oldRoomId);
          if (newRoomId) ws.subscribe(newRoomId);
        }

        executeActions(actions, handle);
      },

      close(ws) {
        const handle = asWsHandle(ws);
        const peerId = socketIdToPeer.get(handle.id);
        if (!peerId) return;

        const roomId = roomManager.getPeerRoom(peerId);
        roomManager.removePeer(peerId);

        // Notify room of peer departure
        if (roomId) {
          ws.publish(roomId, serializeServerMessage({
            type: "peer-left",
            peerId,
          }));
        }

        // Clean up all per-connection state
        peerSockets.delete(peerId);
        socketIdToPeer.delete(handle.id);
        rateLimits.delete(handle.id);
        logger.debug({ peerId }, "Peer disconnected");
      },
    });
}
