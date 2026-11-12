// Builds the website into dist/, which the Rust API serves at /. In development (npm run dev)
// the API routes are proxied to the API on :8000, so the site and the API share one origin.
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const API = process.env.SCAM_MAP_API_URL ?? "http://127.0.0.1:8000";
const API_ROUTES = ["/health", "/reports", "/lookup", "/map", "/alerts", "/challenges", "/license", "/app"];

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    // The shared contract files (areas, scam types, vectors) live next to ui/.
    fs: { allow: [".."] },
    proxy: Object.fromEntries(API_ROUTES.map((route) => [route, API])),
  },
  build: { outDir: "dist", sourcemap: false },
  test: { environment: "jsdom", include: ["src/**/*.test.{ts,tsx}"] },
});
