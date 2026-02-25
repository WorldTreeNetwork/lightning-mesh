import { describe, test, expect, beforeEach } from "bun:test";
import { handleMessage } from "../signaling/handlers";
import { parseClientMessage } from "../signaling/messages";
import { RoomManager } from "../rooms/RoomManager";
import { SendAction, PublishAction, SignalingAction } from "../types";

// Helpers to narrow action types
function isSend(a: SignalingAction): a is SendAction {
  return a.type === "send";
}
function isPublish(a: SignalingAction): a is PublishAction {
  return a.type === "publish";
}

describe("handleMessage", () => {
  let rm: RoomManager;

  beforeEach(() => {
    rm = new RoomManager();
  });

  // -------------------------------------------------------------------------
  describe("join-room", () => {
    test("returns SendAction(room-joined) + PublishAction(peer-joined) for first peer", () => {
      const actions = handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);

      expect(actions).toHaveLength(2);

      const send = actions.find(isSend);
      expect(send).toBeDefined();
      expect(send!.targetPeerId).toBe("peer-1");
      expect(send!.message.type).toBe("room-joined");
      if (send!.message.type === "room-joined") {
        expect(send!.message.roomId).toBe("room-a");
        expect(send!.message.peers).toHaveLength(0); // no other peers yet
      }

      const pub = actions.find(isPublish);
      expect(pub).toBeDefined();
      expect(pub!.roomId).toBe("room-a");
      expect(pub!.message.type).toBe("peer-joined");
      if (pub!.message.type === "peer-joined") {
        expect(pub!.message.peerId).toBe("peer-1");
        expect(pub!.message.displayName).toBe("Alice");
      }
      expect(pub!.excludePeerId).toBe("peer-1");
    });

    test("room-joined includes existing peers when second peer joins", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      const actions = handleMessage("peer-2", { type: "join-room", roomId: "room-a", displayName: "Bob" }, rm);

      const send = actions.find(isSend);
      expect(send).toBeDefined();
      expect(send!.message.type).toBe("room-joined");
      if (send!.message.type === "room-joined") {
        expect(send!.message.peers).toHaveLength(1);
        expect(send!.message.peers[0].peerId).toBe("peer-1");
        expect(send!.message.peers[0].displayName).toBe("Alice");
      }
    });

    test("joining same room twice returns SendAction(error, ALREADY_IN_ROOM)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      const actions = handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);

      expect(actions).toHaveLength(1);
      expect(actions[0].type).toBe("send");
      const send = actions[0] as SendAction;
      expect(send.targetPeerId).toBe("peer-1");
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("ALREADY_IN_ROOM");
      }
    });

    test("implicit leave: joining new room emits peer-left to old room, then room-joined + peer-joined to new room", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-a", displayName: "Bob" }, rm);

      const actions = handleMessage("peer-1", { type: "join-room", roomId: "room-b", displayName: "Alice" }, rm);

      // Should have 3 actions: publish(peer-left to old), send(room-joined), publish(peer-joined to new)
      expect(actions).toHaveLength(3);

      const peerLeftPub = actions.find(
        (a) => isPublish(a) && a.message.type === "peer-left"
      ) as PublishAction | undefined;
      expect(peerLeftPub).toBeDefined();
      expect(peerLeftPub!.roomId).toBe("room-a");
      if (peerLeftPub!.message.type === "peer-left") {
        expect(peerLeftPub!.message.peerId).toBe("peer-1");
      }

      const send = actions.find(isSend) as SendAction | undefined;
      expect(send).toBeDefined();
      expect(send!.message.type).toBe("room-joined");
      if (send!.message.type === "room-joined") {
        expect(send!.message.roomId).toBe("room-b");
      }

      const peerJoinedPub = actions.find(
        (a) => isPublish(a) && a.message.type === "peer-joined"
      ) as PublishAction | undefined;
      expect(peerJoinedPub).toBeDefined();
      expect(peerJoinedPub!.roomId).toBe("room-b");
      if (peerJoinedPub!.message.type === "peer-joined") {
        expect(peerJoinedPub!.message.peerId).toBe("peer-1");
      }
    });
  });

  // -------------------------------------------------------------------------
  describe("leave-room", () => {
    test("returns PublishAction(peer-left) when peer is in a room", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-a", displayName: "Bob" }, rm);

      const actions = handleMessage("peer-1", { type: "leave-room" }, rm);

      expect(actions).toHaveLength(1);
      expect(actions[0].type).toBe("publish");
      const pub = actions[0] as PublishAction;
      expect(pub.roomId).toBe("room-a");
      expect(pub.message.type).toBe("peer-left");
      if (pub.message.type === "peer-left") {
        expect(pub.message.peerId).toBe("peer-1");
      }
    });

    test("returns SendAction(error, NOT_IN_ROOM) when peer is not in a room", () => {
      const actions = handleMessage("peer-1", { type: "leave-room" }, rm);

      expect(actions).toHaveLength(1);
      expect(actions[0].type).toBe("send");
      const send = actions[0] as SendAction;
      expect(send.targetPeerId).toBe("peer-1");
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("NOT_IN_ROOM");
      }
    });
  });

  // -------------------------------------------------------------------------
  describe("offer relay", () => {
    test("relays offer to target with fromPeerId set", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-a", displayName: "Bob" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "offer", targetPeerId: "peer-2", sdp: "v=0\r\n..." },
        rm
      );

      expect(actions).toHaveLength(1);
      expect(actions[0].type).toBe("send");
      const send = actions[0] as SendAction;
      expect(send.targetPeerId).toBe("peer-2");
      expect(send.message.type).toBe("offer");
      if (send.message.type === "offer") {
        expect(send.message.fromPeerId).toBe("peer-1");
        expect(send.message.sdp).toBe("v=0\r\n...");
      }
    });

    test("offer to self returns SendAction(error, PEER_NOT_FOUND)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "offer", targetPeerId: "peer-1", sdp: "v=0\r\n..." },
        rm
      );

      expect(actions).toHaveLength(1);
      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("PEER_NOT_FOUND");
      }
    });

    test("offer when not in room returns SendAction(error, NOT_IN_ROOM)", () => {
      const actions = handleMessage(
        "peer-1",
        { type: "offer", targetPeerId: "peer-2", sdp: "v=0\r\n..." },
        rm
      );

      expect(actions).toHaveLength(1);
      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("NOT_IN_ROOM");
      }
    });

    test("offer to peer in different room returns SendAction(error, PEER_NOT_FOUND)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-b", displayName: "Bob" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "offer", targetPeerId: "peer-2", sdp: "v=0\r\n..." },
        rm
      );

      expect(actions).toHaveLength(1);
      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("PEER_NOT_FOUND");
      }
    });
  });

  // -------------------------------------------------------------------------
  describe("answer relay", () => {
    test("relays answer to target with fromPeerId set", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-a", displayName: "Bob" }, rm);

      const actions = handleMessage(
        "peer-2",
        { type: "answer", targetPeerId: "peer-1", sdp: "v=0\r\nanswer..." },
        rm
      );

      expect(actions).toHaveLength(1);
      const send = actions[0] as SendAction;
      expect(send.targetPeerId).toBe("peer-1");
      expect(send.message.type).toBe("answer");
      if (send.message.type === "answer") {
        expect(send.message.fromPeerId).toBe("peer-2");
        expect(send.message.sdp).toBe("v=0\r\nanswer...");
      }
    });

    test("answer to self returns SendAction(error, PEER_NOT_FOUND)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "answer", targetPeerId: "peer-1", sdp: "v=0\r\n..." },
        rm
      );

      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("PEER_NOT_FOUND");
      }
    });

    test("answer when not in room returns SendAction(error, NOT_IN_ROOM)", () => {
      const actions = handleMessage(
        "peer-1",
        { type: "answer", targetPeerId: "peer-2", sdp: "v=0\r\n..." },
        rm
      );

      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("NOT_IN_ROOM");
      }
    });

    test("answer to peer in different room returns SendAction(error, PEER_NOT_FOUND)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-b", displayName: "Bob" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "answer", targetPeerId: "peer-2", sdp: "v=0\r\n..." },
        rm
      );

      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("PEER_NOT_FOUND");
      }
    });
  });

  // -------------------------------------------------------------------------
  describe("ice-candidate relay", () => {
    test("relays ice-candidate to target with fromPeerId set", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-a", displayName: "Bob" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "ice-candidate", targetPeerId: "peer-2", candidate: "candidate:1 1 UDP..." },
        rm
      );

      expect(actions).toHaveLength(1);
      const send = actions[0] as SendAction;
      expect(send.targetPeerId).toBe("peer-2");
      expect(send.message.type).toBe("ice-candidate");
      if (send.message.type === "ice-candidate") {
        expect(send.message.fromPeerId).toBe("peer-1");
        expect(send.message.candidate).toBe("candidate:1 1 UDP...");
      }
    });

    test("ice-candidate to self returns SendAction(error, PEER_NOT_FOUND)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "ice-candidate", targetPeerId: "peer-1", candidate: "candidate:..." },
        rm
      );

      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("PEER_NOT_FOUND");
      }
    });

    test("ice-candidate when not in room returns SendAction(error, NOT_IN_ROOM)", () => {
      const actions = handleMessage(
        "peer-1",
        { type: "ice-candidate", targetPeerId: "peer-2", candidate: "candidate:..." },
        rm
      );

      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("NOT_IN_ROOM");
      }
    });

    test("ice-candidate to peer in different room returns SendAction(error, PEER_NOT_FOUND)", () => {
      handleMessage("peer-1", { type: "join-room", roomId: "room-a", displayName: "Alice" }, rm);
      handleMessage("peer-2", { type: "join-room", roomId: "room-b", displayName: "Bob" }, rm);

      const actions = handleMessage(
        "peer-1",
        { type: "ice-candidate", targetPeerId: "peer-2", candidate: "candidate:..." },
        rm
      );

      const send = actions[0] as SendAction;
      expect(send.message.type).toBe("error");
      if (send.message.type === "error") {
        expect(send.message.code).toBe("PEER_NOT_FOUND");
      }
    });
  });

  // -------------------------------------------------------------------------
  describe("ping", () => {
    test("returns SendAction(pong) back to the sender", () => {
      const actions = handleMessage("peer-1", { type: "ping" }, rm);

      expect(actions).toHaveLength(1);
      expect(actions[0].type).toBe("send");
      const send = actions[0] as SendAction;
      expect(send.targetPeerId).toBe("peer-1");
      expect(send.message.type).toBe("pong");
    });
  });
});

// -----------------------------------------------------------------------------
// Valid UUID v4 for use in parseClientMessage tests
const VALID_UUID = "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11";

describe("parseClientMessage", () => {
  describe("valid inputs", () => {
    test("parses join-room with roomId", () => {
      const msg = parseClientMessage({ type: "join-room", roomId: "room-a" });
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("join-room");
      if (msg!.type === "join-room") {
        expect(msg.roomId).toBe("room-a");
        expect(msg.displayName).toBeUndefined();
      }
    });

    test("parses join-room with displayName", () => {
      const msg = parseClientMessage({ type: "join-room", roomId: "room-a", displayName: "Alice" });
      expect(msg).not.toBeNull();
      if (msg!.type === "join-room") {
        expect(msg.displayName).toBe("Alice");
      }
    });

    test("parses join-room from JSON string", () => {
      const msg = parseClientMessage(JSON.stringify({ type: "join-room", roomId: "room-a" }));
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("join-room");
    });

    test("parses leave-room", () => {
      const msg = parseClientMessage({ type: "leave-room" });
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("leave-room");
    });

    test("parses offer", () => {
      const msg = parseClientMessage({ type: "offer", targetPeerId: VALID_UUID, sdp: "v=0\r\n" });
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("offer");
      if (msg!.type === "offer") {
        expect(msg.targetPeerId).toBe(VALID_UUID);
        expect(msg.sdp).toBe("v=0\r\n");
      }
    });

    test("parses answer", () => {
      const msg = parseClientMessage({ type: "answer", targetPeerId: VALID_UUID, sdp: "v=0\r\n" });
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("answer");
    });

    test("parses ice-candidate", () => {
      const msg = parseClientMessage({
        type: "ice-candidate",
        targetPeerId: VALID_UUID,
        candidate: "candidate:1 1 UDP...",
      });
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("ice-candidate");
      if (msg!.type === "ice-candidate") {
        expect(msg.candidate).toBe("candidate:1 1 UDP...");
      }
    });

    test("parses ping", () => {
      const msg = parseClientMessage({ type: "ping" });
      expect(msg).not.toBeNull();
      expect(msg!.type).toBe("ping");
    });
  });

  describe("invalid inputs", () => {
    test("returns null for malformed JSON string", () => {
      expect(parseClientMessage("{not valid json}")).toBeNull();
    });

    test("returns null for null", () => {
      expect(parseClientMessage(null)).toBeNull();
    });

    test("returns null for a number", () => {
      expect(parseClientMessage(42)).toBeNull();
    });

    test("returns null for an array", () => {
      expect(parseClientMessage([])).toBeNull();
    });

    test("returns null for object missing type", () => {
      expect(parseClientMessage({ roomId: "room-a" })).toBeNull();
    });

    test("returns null for unknown type", () => {
      expect(parseClientMessage({ type: "unknown-action" })).toBeNull();
    });

    test("returns null for join-room missing roomId", () => {
      expect(parseClientMessage({ type: "join-room" })).toBeNull();
    });

    test("returns null for offer missing sdp", () => {
      expect(parseClientMessage({ type: "offer", targetPeerId: VALID_UUID })).toBeNull();
    });

    test("returns null for offer missing targetPeerId", () => {
      expect(parseClientMessage({ type: "offer", sdp: "v=0\r\n" })).toBeNull();
    });

    test("returns null for ice-candidate missing candidate", () => {
      expect(parseClientMessage({ type: "ice-candidate", targetPeerId: VALID_UUID })).toBeNull();
    });

    test("returns null for answer missing sdp", () => {
      expect(parseClientMessage({ type: "answer", targetPeerId: VALID_UUID })).toBeNull();
    });
  });
});
