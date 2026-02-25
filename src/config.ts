export const config = {
  port: parseInt(process.env.PORT || "3000", 10),
  logLevel: (process.env.LOG_LEVEL || "info") as string,
  adminToken: process.env.ADMIN_TOKEN || "",
  rateLimitPerSecond: parseInt(process.env.RATE_LIMIT || "30", 10),
};
