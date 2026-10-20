import { ensureDocumentApi } from "./document-api.mjs";

const api = await ensureDocumentApi();
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    api.stop();
    process.exit(0);
  });
}
