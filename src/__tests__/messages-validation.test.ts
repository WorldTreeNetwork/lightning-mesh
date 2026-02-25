import { describe, test, expect } from "bun:test";
import { parseClientMessage } from "../signaling/messages";

const VALID_UUID = "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11";

// Helper to build a minimal join-room message
function joinRoom(roomId: string, displayName?: string): unknown {
  const msg: Record<string, string> = { type: "join-room", roomId };
  if (displayName !== undefined) msg.displayName = displayName;
  return msg;
}

// Helper to build an offer/answer message
function sdpMsg(type: "offer" | "answer", targetPeerId: string, sdp: string): unknown {
  return { type, targetPeerId, sdp };
}

// Helper to build an ice-candidate message
function iceMsg(targetPeerId: string, candidate: string): unknown {
  return { type: "ice-candidate", targetPeerId, candidate };
}

describe("parseClientMessage - roomId validation", () => {
  test("returns null for roomId with spaces", () => {
    expect(parseClientMessage(joinRoom("room with spaces"))).toBeNull();
  });

  test("returns null for roomId with path traversal", () => {
    expect(parseClientMessage(joinRoom("../../etc"))).toBeNull();
  });

  test("returns null for roomId with special chars @#$", () => {
    expect(parseClientMessage(joinRoom("room@#$"))).toBeNull();
  });

  test("returns null for roomId longer than 64 characters", () => {
    expect(parseClientMessage(joinRoom("a".repeat(65)))).toBeNull();
  });

  test("returns null for empty roomId", () => {
    expect(parseClientMessage(joinRoom(""))).toBeNull();
  });

  test("accepts roomId with hyphens and underscores", () => {
    const result = parseClientMessage(joinRoom("my-room_123"));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("join-room");
  });

  test("accepts roomId exactly 64 characters", () => {
    const result = parseClientMessage(joinRoom("a".repeat(64)));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("join-room");
  });
});

describe("parseClientMessage - displayName validation", () => {
  test("returns null for displayName longer than 128 characters", () => {
    expect(parseClientMessage(joinRoom("valid-room", "a".repeat(129)))).toBeNull();
  });

  test("accepts displayName exactly 128 characters", () => {
    const result = parseClientMessage(joinRoom("valid-room", "a".repeat(128)));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("join-room");
  });
});

describe("parseClientMessage - sdp validation", () => {
  test("returns null for sdp longer than 16384 characters (offer)", () => {
    expect(parseClientMessage(sdpMsg("offer", VALID_UUID, "a".repeat(16385)))).toBeNull();
  });

  test("returns null for sdp longer than 16384 characters (answer)", () => {
    expect(parseClientMessage(sdpMsg("answer", VALID_UUID, "a".repeat(16385)))).toBeNull();
  });

  test("accepts sdp exactly 16384 characters (offer)", () => {
    const result = parseClientMessage(sdpMsg("offer", VALID_UUID, "a".repeat(16384)));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("offer");
  });

  test("accepts sdp exactly 16384 characters (answer)", () => {
    const result = parseClientMessage(sdpMsg("answer", VALID_UUID, "a".repeat(16384)));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("answer");
  });
});

describe("parseClientMessage - candidate validation", () => {
  test("returns null for candidate longer than 1024 characters", () => {
    expect(parseClientMessage(iceMsg(VALID_UUID, "a".repeat(1025)))).toBeNull();
  });

  test("accepts candidate exactly 1024 characters", () => {
    const result = parseClientMessage(iceMsg(VALID_UUID, "a".repeat(1024)));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("ice-candidate");
  });
});

describe("parseClientMessage - targetPeerId UUID validation", () => {
  test("returns null for targetPeerId that is not a UUID", () => {
    expect(parseClientMessage(sdpMsg("offer", "not-a-uuid", "sdp-data"))).toBeNull();
  });

  test("returns null for empty targetPeerId", () => {
    expect(parseClientMessage(sdpMsg("offer", "", "sdp-data"))).toBeNull();
  });

  test("returns null for UUID with wrong version (version 3 instead of 4)", () => {
    // Version marker is '3' instead of '4' at position 14
    expect(parseClientMessage(sdpMsg("offer", "a0eebc99-9c0b-3ef8-bb6d-6bb9bd380a11", "sdp-data"))).toBeNull();
  });

  test("accepts valid UUID v4 targetPeerId", () => {
    const result = parseClientMessage(sdpMsg("offer", VALID_UUID, "sdp-data"));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("offer");
  });

  test("returns null for non-UUID targetPeerId in ice-candidate", () => {
    expect(parseClientMessage(iceMsg("not-a-uuid", "candidate-data"))).toBeNull();
  });

  test("accepts valid UUID v4 targetPeerId in ice-candidate", () => {
    const result = parseClientMessage(iceMsg(VALID_UUID, "candidate-data"));
    expect(result).not.toBeNull();
    expect(result?.type).toBe("ice-candidate");
  });
});
