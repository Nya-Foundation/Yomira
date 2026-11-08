import type { ApiErrorKind } from "../types";
// Type-only: a value import would cycle through i18n -> settings -> transport.
import type { TKey } from "../i18n";

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

/**
 * What to tell a user about a failure, and what they can do about it.
 *
 * The backend distinguishes eleven kinds of failure and the interface used to
 * collapse all of them into "could not be loaded" with a Retry button — which
 * is wrong advice for a quota error, useless for an expired session, and
 * silent about the downloaded galleries that are still readable offline.
 *
 * `eh` is the one kind whose message is the site's own words, so it passes
 * through; every other message is internal Rust prose and is replaced here.
 */
export function errorCopy(e: unknown): { desc: TKey; action?: "signin" | "offline" } {
  const kind = e instanceof ApiError ? e.kind : undefined;
  switch (kind) {
    case "network":
      return { desc: "error.network", action: "offline" };
    case "notSignedIn":
      return { desc: "error.notSignedIn", action: "signin" };
    case "cookie":
      return { desc: "error.cookie", action: "signin" };
    case "image509":
    case "bandwidthExceeded":
      return { desc: "error.quota" };
    case "db":
      return { desc: "error.db" };
    case "pageUnavailable":
      return { desc: "error.pageUnavailable" };
    case "cancelled":
      return { desc: "error.cancelled" };
    default:
      return { desc: "error.site" };
  }
}

/** The site's own message, when it sent one worth showing. */
export function siteMessage(e: unknown): string | null {
  return e instanceof ApiError && e.kind === "eh" && e.message ? e.message : null;
}
