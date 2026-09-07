import type { ApiErrorKind } from "../types";

export class ApiError extends Error {
  code: number;
  kind?: ApiErrorKind;
  constructor(code: number, message: string, kind?: ApiErrorKind) {
    super(message);
    this.code = code;
    this.kind = kind;
  }
}

const ERROR_KINDS: readonly ApiErrorKind[] = [
  "network",
  "httpStatus",
  "parse",
  "notSignedIn",
  "image509",
  "bandwidthExceeded",
  "cookie",
  "db",
  "pageUnavailable",
  "cancelled",
  "eh",
];

function kindOf(value: unknown): ApiErrorKind | null {
  if (typeof value !== "string") return null;
  return (ERROR_KINDS as readonly string[]).includes(value)
    ? (value as ApiErrorKind)
    : null;
}

function parseKindMessage(text: string): { kind: ApiErrorKind; message: string } | null {
  const start = text.indexOf("{");
  if (start === -1) return null;
  try {
    const obj: unknown = JSON.parse(text.slice(start));
    if (obj && typeof obj === "object") {
      const kind = kindOf((obj as Record<string, unknown>).kind);
      if (kind) {
        const message = (obj as Record<string, unknown>).message;
        return { kind, message: typeof message === "string" ? message : text };
      }
    }
  } catch {
    /* not JSON — fall through */
  }
  return null;
}

function extractKindMessage(e: unknown): { kind: ApiErrorKind; message: string } | null {
  if (e && typeof e === "object") {
    const kind = kindOf((e as Record<string, unknown>).kind);
    if (kind) {
      const message = (e as Record<string, unknown>).message;
      return { kind, message: typeof message === "string" ? message : String(e) };
    }
  }
  if (typeof e === "string") return parseKindMessage(e) ?? { kind: "eh", message: e };
  if (e instanceof Error) return parseKindMessage(e.message) ?? { kind: "eh", message: e.message };
  return null;
}

export function normalizeError(e: unknown): ApiError {
  if (e instanceof ApiError) return e;
  const extracted = extractKindMessage(e);
  if (extracted) {
    const code =
      extracted.kind === "image509" || extracted.kind === "bandwidthExceeded" ? 509 : 0;
    return new ApiError(code, extracted.message, extracted.kind);
  }
  return new ApiError(0, e instanceof Error ? e.message : String(e));
}

export function isNotSignedIn(e: unknown): boolean {
  return e instanceof ApiError && e.kind === "notSignedIn";
}
