import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import { startApp } from "./app/app.ts";
import { createHost } from "./host/create-host.ts";

createHost()
  .then((host) => startApp(host))
  .catch((error: unknown) => {
    const message = error instanceof Error ? error.message : "BundlePad 启动失败";
    document.body.textContent = message;
  });
