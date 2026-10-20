import type { AppError } from "./types.ts";

export function asAppError(error: unknown): AppError {
  if (typeof error === "string") {
    const parsed = parseError(error);
    if (parsed) return parsed;
    return { code: "error", message: error };
  }
  if (error && typeof error === "object") {
    const record = error as Record<string, unknown>;
    if (typeof record.message === "string") {
      return {
        code: typeof record.code === "string" ? record.code : "error",
        message: record.message,
      };
    }
  }
  return { code: "error", message: "发生未知错误" };
}

function parseError(value: string): AppError | null {
  try {
    const parsed = JSON.parse(value) as Record<string, unknown>;
    if (typeof parsed.message === "string") {
      return {
        code: typeof parsed.code === "string" ? parsed.code : "error",
        message: parsed.message,
      };
    }
  } catch {
    return null;
  }
  return null;
}
