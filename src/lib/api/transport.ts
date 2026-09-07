import { invoke } from "@tauri-apps/api/core";
import { normalizeError } from "./errors";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Browser fixtures are never loaded on the native command path. */
export const loadMock = () => import("../mock");

/** Invoke a Tauri command and normalize any rejection into an ApiError. */
export async function invokeOrThrow<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw normalizeError(e);
  }
}

export const loadPreferences = () => import("../mock/preferences");
