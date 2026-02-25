import { describe, test, expect, beforeAll, afterAll, afterEach } from "bun:test";
import { Elysia } from "elysia";
import { RoomManager } from "../rooms/RoomManager";
import { createSignalingWs } from "../ws/setup";
import { createHealthRoutes } from "../health/routes";

let app: Elysia;
let baseUrl: string;
let wsUrl: string;

// Track all clients created per test for cleanup
const activeClients: Array<{ close: () => void }> = [];

type Client = {
  ws: WebSocket;
  messages: any[];
  waitForMessage: (predicate: (msg: any) => boolean, timeoutMs?: number) => Promise<any>;
  close: () => void;
};

function connectClient(): Promise<Client> {
  return new Promise((resolve, reject) => {
    const messages: any[] = [];
    const waiters: Array<{
      predicate: (msg: any) => boolean;
      resolve: (msg: any) => void;
      reject: (err: Error) => void;
      timer: ReturnType<typeof setTimeout>;
    }> = [];

    const ws = new WebSocket(wsUrl);

    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data as string);
      messages.push(msg);
      for (let i = waiters.length - 1; i >= 0; i--) {
        if (waiters[i].predicate(msg)) {
          clearTimeout(waiters[i].timer);
          waiters[i].resolve(msg);
          waiters.splice(i, 1);
        }
      }
    };

    ws.onerror = () => reject(new Error("WebSocket connection error"));

    ws.onopen = () => {
      const client: Client = {
        ws,
        messages,
        waitForMessage: (predicate, timeoutMs = 5000) =>
          new Promise((res, rej) => {
            const existing = messages.find(predicate);
            if (existing) { res(existing); return; }
            const timer = setTimeout(() => {
              const idx = waiters.findIndex(w => w.resolve === res);
              if (idx !== -1) waiters.splice(idx, 1);
              rej(new Error(`waitForMessage timed out after ${timeoutMs}ms`));
            }, timeoutMs);
            waiters.push({ predicate, resolve: res, reject: rej, timer });
          }),
        close: () => { ws.close(); },
      };
      activeClients.push(client);
      resolve(client);
    };
  });
}

const delay = (ms: number) => new Promise(r => setTimeout(r, ms));

beforeAll(() => {
  const roomManager = new RoomManager();
  app = new Elysia()
    .use(createSignalingWs(roomManager))
    .use(createHealthRoutes(roomManager))
    .listen(0);
  const port = app.server!.port;
  baseUrl = `http://localhost:${port}`;
  wsUrl = `ws://localhost:${port}/ws`;
});

afterAll(() => {
  app.stop();
});

afterEach(async () => {
  for (const client of activeClients) {
    try { client.close(); } catch { /* ignore */ }
  }
  activeClients.length = 0;
  await delay(50);
});

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

describe("Integration: WebSocket signaling", () => {
  test("1. Connection and Welcome - receives welcome with valid UUID peerId", async () => {
    const client = await connectClient();
    const welcome = await client.waitForMessage(m => m.type === "welcome");
    expect(welcome.type).toBe("welcome");
    expect(typeof welcome.peerId).toBe("string");
    expect(UUID_RE.test(welcome.peerId)).toBe(true);
  });

  test("2. Join Room - receives room-joined with empty peers list", async () => {
    const client = await connectClient();
    await client.waitForMessage(m => m.type === "welcome");

    client.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-2" }));

    const joined = await client.waitForMessage(m => m.type === "room-joined");
    expect(joined.type).toBe("room-joined");
    expect(joined.roomId).toBe("test-room-2");
    expect(Array.isArray(joined.peers)).toBe(true);
    expect(joined.peers.length).toBe(0);
  });

  test("3. Second Peer Joins - B's room-joined includes A, A gets peer-joined", async () => {
    const clientA = await connectClient();
    const welcomeA = await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-3" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    const clientB = await connectClient();
    const welcomeB = await clientB.waitForMessage(m => m.type === "welcome");
    clientB.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-3" }));

    // B receives room-joined with A in the peer list
    const joinedB = await clientB.waitForMessage(m => m.type === "room-joined");
    expect(joinedB.type).toBe("room-joined");
    expect(joinedB.roomId).toBe("test-room-3");
    expect(joinedB.peers.length).toBe(1);
    expect(joinedB.peers[0].peerId).toBe(welcomeA.peerId);
    expect(typeof joinedB.peers[0].displayName).toBe("string");

    // A receives peer-joined notification for B
    const peerJoined = await clientA.waitForMessage(m => m.type === "peer-joined");
    expect(peerJoined.peerId).toBe(welcomeB.peerId);
  });

  test("4. SDP Offer/Answer Exchange - relay between peers in same room", async () => {
    const clientA = await connectClient();
    const welcomeA = await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-4" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    const clientB = await connectClient();
    const welcomeB = await clientB.waitForMessage(m => m.type === "welcome");
    clientB.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-4" }));
    await clientB.waitForMessage(m => m.type === "room-joined");

    // Small delay for both peers to be fully registered in server state
    await delay(50);

    // A sends offer to B
    clientA.ws.send(JSON.stringify({ type: "offer", targetPeerId: welcomeB.peerId, sdp: "offer-sdp" }));
    const offerAtB = await clientB.waitForMessage(m => m.type === "offer");
    expect(offerAtB.fromPeerId).toBe(welcomeA.peerId);
    expect(offerAtB.sdp).toBe("offer-sdp");

    // B sends answer to A
    clientB.ws.send(JSON.stringify({ type: "answer", targetPeerId: welcomeA.peerId, sdp: "answer-sdp" }));
    const answerAtA = await clientA.waitForMessage(m => m.type === "answer");
    expect(answerAtA.fromPeerId).toBe(welcomeB.peerId);
    expect(answerAtA.sdp).toBe("answer-sdp");
  });

  test("5. ICE Candidate Relay - candidate forwarded with correct fromPeerId", async () => {
    const clientA = await connectClient();
    const welcomeA = await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-5" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    const clientB = await connectClient();
    const welcomeB = await clientB.waitForMessage(m => m.type === "welcome");
    clientB.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-5" }));
    await clientB.waitForMessage(m => m.type === "room-joined");

    await delay(50);

    clientA.ws.send(JSON.stringify({ type: "ice-candidate", targetPeerId: welcomeB.peerId, candidate: "candidate-data" }));
    const iceAtB = await clientB.waitForMessage(m => m.type === "ice-candidate");
    expect(iceAtB.fromPeerId).toBe(welcomeA.peerId);
    expect(iceAtB.candidate).toBe("candidate-data");
  });

  test("6. Peer Disconnect Notification - A receives peer-left when B disconnects", async () => {
    const clientA = await connectClient();
    const welcomeA = await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-6" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    const clientB = await connectClient();
    const welcomeB = await clientB.waitForMessage(m => m.type === "welcome");
    clientB.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-6" }));
    await clientB.waitForMessage(m => m.type === "room-joined");

    await delay(50);

    clientB.close();

    const peerLeft = await clientA.waitForMessage(m => m.type === "peer-left");
    expect(peerLeft.type).toBe("peer-left");
    expect(peerLeft.peerId).toBe(welcomeB.peerId);
  });

  test("7. Room Cleanup After All Leave - /rooms shows room gone after last peer disconnects", async () => {
    const clientA = await connectClient();
    await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-7" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    // Verify room exists while peer is in it
    const resBefore = await fetch(`${baseUrl}/rooms/test-room-7`);
    const before = await resBefore.json() as any;
    expect(before.roomId).toBe("test-room-7");
    expect(before.peerCount).toBe(1);

    clientA.close();
    await delay(100);

    // Room should be cleaned up after all peers leave
    const resAfter = await fetch(`${baseUrl}/rooms`);
    const after = await resAfter.json() as any;
    const roomIds: string[] = after.rooms.map((r: any) => r.roomId);
    expect(roomIds.includes("test-room-7")).toBe(false);
  });

  test("8. Health Endpoints - /health, /rooms, /rooms/:roomId", async () => {
    // GET /health
    const healthRes = await fetch(`${baseUrl}/health`);
    expect(healthRes.status).toBe(200);
    const health = await healthRes.json() as any;
    expect(health.status).toBe("ok");
    expect(typeof health.uptime).toBe("number");
    expect(health.uptime).toBeGreaterThanOrEqual(0);

    // Set up a peer in a room
    const client = await connectClient();
    await client.waitForMessage(m => m.type === "welcome");
    client.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-8" }));
    await client.waitForMessage(m => m.type === "room-joined");
    await delay(50);

    // GET /rooms
    const roomsRes = await fetch(`${baseUrl}/rooms`);
    expect(roomsRes.status).toBe(200);
    const rooms = await roomsRes.json() as any;
    expect(typeof rooms.totalRooms).toBe("number");
    expect(typeof rooms.totalPeers).toBe("number");
    const found = rooms.rooms.find((r: any) => r.roomId === "test-room-8");
    expect(found).toBeDefined();
    expect(found.peerCount).toBe(1);

    // GET /rooms/:roomId
    const roomRes = await fetch(`${baseUrl}/rooms/test-room-8`);
    expect(roomRes.status).toBe(200);
    const room = await roomRes.json() as any;
    expect(room.roomId).toBe("test-room-8");
    expect(Array.isArray(room.peers)).toBe(true);
    expect(room.peers.length).toBe(1);
    expect(typeof room.peers[0].peerId).toBe("string");
    expect(UUID_RE.test(room.peers[0].peerId)).toBe(true);
    expect(typeof room.peers[0].joinedAt).toBe("number");
    expect(room.peerCount).toBe(1);
    expect(typeof room.createdAt).toBe("number");
  });

  test("9. Invalid Message - malformed JSON returns INVALID_MESSAGE error", async () => {
    const client = await connectClient();
    await client.waitForMessage(m => m.type === "welcome");

    client.ws.send("not valid json {{{");

    const error = await client.waitForMessage(m => m.type === "error");
    expect(error.type).toBe("error");
    expect(error.code).toBe("INVALID_MESSAGE");
  });

  test("10. Ping/Pong - send ping, receive pong", async () => {
    const client = await connectClient();
    await client.waitForMessage(m => m.type === "welcome");

    client.ws.send(JSON.stringify({ type: "ping" }));

    const pong = await client.waitForMessage(m => m.type === "pong");
    expect(pong.type).toBe("pong");
  });

  test("11. 404 for Non-Existent Room - GET /rooms/:roomId returns 404 with error body", async () => {
    const res = await fetch(`${baseUrl}/rooms/nonexistent-room-xyz`);
    expect(res.status).toBe(404);
    const body = await res.json() as any;
    expect(body.error).toBe("Room not found");
  });

  test("12. Three-Peer Room - C's room-joined includes A and B, A and B get peer-joined for C", async () => {
    const clientA = await connectClient();
    const welcomeA = await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-12" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    const clientB = await connectClient();
    const welcomeB = await clientB.waitForMessage(m => m.type === "welcome");
    clientB.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-12" }));
    await clientB.waitForMessage(m => m.type === "room-joined");
    await clientA.waitForMessage(m => m.type === "peer-joined" && m.peerId === welcomeB.peerId);

    const clientC = await connectClient();
    const welcomeC = await clientC.waitForMessage(m => m.type === "welcome");
    clientC.ws.send(JSON.stringify({ type: "join-room", roomId: "test-room-12" }));

    // C's room-joined includes A and B in peer list
    const joinedC = await clientC.waitForMessage(m => m.type === "room-joined");
    expect(joinedC.peers.length).toBe(2);
    const peerIds = joinedC.peers.map((p: any) => p.peerId);
    expect(peerIds).toContain(welcomeA.peerId);
    expect(peerIds).toContain(welcomeB.peerId);

    // A and B both receive peer-joined for C
    const peerJoinedAtA = await clientA.waitForMessage(m => m.type === "peer-joined" && m.peerId === welcomeC.peerId);
    expect(peerJoinedAtA.peerId).toBe(welcomeC.peerId);
    const peerJoinedAtB = await clientB.waitForMessage(m => m.type === "peer-joined" && m.peerId === welcomeC.peerId);
    expect(peerJoinedAtB.peerId).toBe(welcomeC.peerId);

    // Verify peer count via HTTP
    await delay(50);
    const res = await fetch(`${baseUrl}/rooms/test-room-12`);
    expect(res.status).toBe(200);
    const room = await res.json() as any;
    expect(room.peerCount).toBe(3);
  });

  test("13. Cross-Room Isolation - offer to peer in different room returns PEER_NOT_FOUND error", async () => {
    const clientA = await connectClient();
    await clientA.waitForMessage(m => m.type === "welcome");
    clientA.ws.send(JSON.stringify({ type: "join-room", roomId: "room-alpha" }));
    await clientA.waitForMessage(m => m.type === "room-joined");

    const clientB = await connectClient();
    const welcomeB = await clientB.waitForMessage(m => m.type === "welcome");
    clientB.ws.send(JSON.stringify({ type: "join-room", roomId: "room-beta" }));
    await clientB.waitForMessage(m => m.type === "room-joined");

    await delay(50);

    // A sends offer targeting B who is in a different room
    clientA.ws.send(JSON.stringify({ type: "offer", targetPeerId: welcomeB.peerId, sdp: "offer-sdp" }));

    const error = await clientA.waitForMessage(m => m.type === "error");
    expect(error.type).toBe("error");
    expect(error.code).toBe("PEER_NOT_FOUND");
  });

  test("14. Input Validation - join-room with invalid roomId returns INVALID_MESSAGE error", async () => {
    const client = await connectClient();
    await client.waitForMessage(m => m.type === "welcome");

    client.ws.send(JSON.stringify({ type: "join-room", roomId: "room with spaces" }));

    const error = await client.waitForMessage(m => m.type === "error");
    expect(error.type).toBe("error");
    expect(error.code).toBe("INVALID_MESSAGE");
  });
});
