import { defineConfig } from "vitest/config";
// @ts-expect-error 模板不安装 @types/node，这里只读取 Tauri 注入的环境变量。
import process from "node:process";
import { attachDocumentApi } from "./scripts/document-api.mjs";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  clearScreen: false,
  plugins: [
    {
      name: "bundlepad-document-api",
      configureServer(server) {
        attachDocumentApi(server);
      },
      configurePreviewServer(server) {
        attachDocumentApi(server);
      },
    },
  ],
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
}));
