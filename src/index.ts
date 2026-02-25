import { Elysia } from "elysia";
import { cors } from "@elysiajs/cors";
import { RoomManager } from "./rooms/RoomManager";
import { createSignalingWs } from "./ws/setup";
import { createHealthRoutes } from "./health/routes";
import { logger } from "./logger";
import { config } from "./config";

const roomManager = new RoomManager();

const app = new Elysia()
  .use(cors())
  .use(createSignalingWs(roomManager))
  .use(createHealthRoutes(roomManager))
  .listen(config.port);

logger.info(
  { host: app.server?.hostname, port: app.server?.port },
  "Signaling server started"
);
