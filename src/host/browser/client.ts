const API_DOWN = {
  code: "error",
  message: "文档服务还没就绪。开发服务器会自动启动它，第一次需要编译，完成后再打开文件。",
};

export async function postJson<T>(path: string, body: unknown = {}): Promise<T> {
  let response: Response;
  try {
    response = await fetch(path, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
  } catch {
    throw API_DOWN;
  }
  const payload: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    if (payload && typeof payload === "object" && "message" in payload) throw payload;
    throw { code: "error", message: "文档服务请求失败" };
  }
  return payload as T;
}
