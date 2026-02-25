import pino from "pino";
import { config } from "./config";

export const logger = pino({
  name: "mjolnir-mesh",
  level: config.logLevel,
});
