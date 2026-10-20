import { spawn } from "node:child_process";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const port = 1422;
const targetDir = path.join(root, "src-tauri", "target-http");

export function attachDocumentApi(server) {
  const handle = ensureDocumentApi();
  server.middlewares.use((req, res, next) => {
    if (!req.url?.startsWith("/api")) {
      next();
      return;
    }
    const headers = { ...req.headers, host: `127.0.0.1:${port}` };
    delete headers.connection;
    const proxy = http.request(
      {
        host: "127.0.0.1",
        port,
        path: req.url,
        method: req.method || "POST",
        headers,
      },
      (incoming) => {
        res.writeHead(incoming.statusCode ?? 502, incoming.headers);
        incoming.pipe(res);
      },
    );
    proxy.setTimeout(0);
    proxy.on("error", (error) => {
      if (res.headersSent) {
        res.destroy(error);
        return;
      }
      res.statusCode = 502;
      res.setHeader("content-type", "application/json; charset=utf-8");
      res.end(
        JSON.stringify({
          code: "error",
          message: "文档服务还没就绪。开发服务器会自动启动它，第一次需要编译，完成后再打开文件。",
        }),
      );
    });
    req.pipe(proxy);
  });
  const stop = () => {
    void handle.then((api) => api.stop());
  };
  server.httpServer?.once("close", stop);
}

export function ensureDocumentApi() {
  return listening(port).then((open) => {
    if (open) {
      console.log(`[bundlepad] 文档服务已在 http://127.0.0.1:${port}`);
      return { stop() {} };
    }
    console.log("[bundlepad] 正在启动文档服务。第一次会编译，完成前浏览器还不能打开文件。");
    const child = spawn(
      "cargo",
      ["run", "--manifest-path", "src-tauri/Cargo.toml", "--bin", "bundlepad-http", "--features", "http"],
      {
        cwd: root,
        stdio: "inherit",
        env: { ...process.env, CARGO_TARGET_DIR: targetDir },
        windowsHide: true,
      },
    );
    let stopped = false;
    child.on("exit", (code) => {
      if (!stopped && code !== 0) console.error(`[bundlepad] 文档服务退出，代码 ${code ?? "未知"}`);
    });
    return {
      stop() {
        stopped = true;
        if (child.pid == null) return;
        if (process.platform === "win32") {
          spawn("taskkill", ["/pid", String(child.pid), "/T", "/F"], { stdio: "ignore", windowsHide: true });
          return;
        }
        child.kill("SIGTERM");
      },
    };
  });
}

function listening(checkPort) {
  return new Promise((resolve) => {
    const socket = net.connect({ host: "127.0.0.1", port: checkPort });
    const finish = (open) => {
      socket.removeAllListeners();
      socket.destroy();
      resolve(open);
    };
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
  });
}
