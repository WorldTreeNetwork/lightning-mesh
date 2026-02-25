import { describe, test, expect, beforeAll, afterAll, afterEach } from "bun:test";
import { Elysia } from "elysia";
import { RoomManager } from "../rooms/RoomManager";
import { createSignalingWs } from "../ws/setup";

let app: Elysia;
let wsUrl: string;

const activeClients: Array<{ close: () => void }> = [];

type Client = {
  ws: WebSocket;
  messages: any[];
  waitForMessage: (predicate: (msg: any) => boolean, timeoutMs?: number) => Promise<any>;
  close: () => void;
};

function connectClient(url: string): Promise<Client> {
  return new Promise((resolve, reject) => {
    const messages: any[] = [];
    const waiters: Array<{
      predicate: (msg: any) => boolean;
      resolve: (msg: any) => void;
      reject: (err: Error) => void;
      timer: ReturnType<typeof setTimeout>;
    }> = [];

    const ws = new WebSocket(url);

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

const delay = (ms: number) => new Promise<void>(r => setTimeout(r, ms));

beforeAll(() => {
  const roomManager = new RoomManager();
  app = new Elysia()
    .use(createSignalingWs(roomManager))
    .listen(0);
  const port = app.server!.port;
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

describe("Rate limiting", () => {
  test("exceeding 30 messages/sec triggers RATE_LIMITED error", async () => {
    const client = await connectClient(wsUrl);
    await client.waitForMessage(m => m.type === "welcome");

    // Send 31 pings rapidly — at least one must be rate limited
    for (let i = 0; i < 31; i++) {
      client.ws.send(JSON.stringify({ type: "ping" }));
    }

    const rateLimited = await client.waitForMessage(
      m => m.type === "error" && m.code === "RATE_LIMITED",
      5000,
    );
    expect(rateLimited.type).toBe("error");
    expect(rateLimited.code).toBe("RATE_LIMITED");
  });

  test("recovers after rate limit window (1100ms)", async () => {
    const client = await connectClient(wsUrl);
    await client.waitForMessage(m => m.type === "welcome");

    // Exhaust the rate limit
    for (let i = 0; i < 31; i++) {
      client.ws.send(JSON.stringify({ type: "ping" }));
    }

    await client.waitForMessage(
      m => m.type === "error" && m.code === "RATE_LIMITED",
      5000,
    );

    // Wait for the sliding window to reset (window is 1000ms, add 100ms buffer)
    await delay(1100);

    // Clear previously collected messages so we only watch for the new pong
    client.messages.length = 0;

    client.ws.send(JSON.stringify({ type: "ping" }));

    const pong = await client.waitForMessage(m => m.type === "pong", 5000);
    expect(pong.type).toBe("pong");
  });

  test("per-connection isolation: two clients at 25 msgs each are not rate limited", async () => {
    const clientA = await connectClient(wsUrl);
    const clientB = await connectClient(wsUrl);

    await clientA.waitForMessage(m => m.type === "welcome");
    await clientB.waitForMessage(m => m.type === "welcome");

    // Send 25 pings from each connection simultaneously (each under the 30/sec limit)
    for (let i = 0; i < 25; i++) {
      clientA.ws.send(JSON.stringify({ type: "ping" }));
      clientB.ws.send(JSON.stringify({ type: "ping" }));
    }

    // Wait briefly for responses to arrive
    await delay(300);

    const aRateLimited = clientA.messages.some(
      m => m.type === "error" && m.code === "RATE_LIMITED",
    );
    const bRateLimited = clientB.messages.some(
      m => m.type === "error" && m.code === "RATE_LIMITED",
    );

    expect(aRateLimited).toBe(false);
    expect(bRateLimited).toBe(false);

    // Both should have received some pongs
    const aPongs = clientA.messages.filter(m => m.type === "pong").length;
    const bPongs = clientB.messages.filter(m => m.type === "pong").length;
    expect(aPongs).toBeGreaterThan(0);
    expect(bPongs).toBeGreaterThan(0);
  });
});
