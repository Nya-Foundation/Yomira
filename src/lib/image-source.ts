import { invokeOrThrow, isTauri } from "./api/transport";

/**
 * Address of the backend's loopback image server, resolved once at startup.
 *
 * Images are addressed by composing this base with the remote URL, so showing
 * a thumbnail costs no IPC and `<img loading="lazy">` keeps its meaning — the
 * browser decides what to request and in what order, and the backend fetches
 * and caches on first request. Only hosts the backend allowlists resolve.
 */
let base: string | null = null;

export async function initImageSource(): Promise<void> {
  if (!isTauri()) return;
  try {
    base = await invokeOrThrow<string>("image_base_url");
  } catch {
    base = null;
  }
}

/** Whether a remote thumbnail URL can be composed yet. */
export function imageBaseReady(): boolean {
  return !isTauri() || base !== null;
}

/**
 * Compose the served URL for a remote image. The whole target is percent
 * encoded into a single `u` value because the server decodes everything after
 * `?u=`; `encodeURIComponent` escapes `&` and `=`, so a query in the target
 * cannot add parameters of its own.
 */
export function remoteImageSrc(base: string, url: string): string {
  return `${base}/remote?u=${encodeURIComponent(url)}`;
}

/** A displayable `src` for a remote image URL, or null when none is possible. */
export function imageSrc(url: string): string | null {
  if (!url) return null;
  // Demo fixtures are data URLs, and saved pages already carry a served URL.
  if (!isTauri() || url.startsWith("http://127.0.0.1:")) return url;
  if (base === null) return null;
  return remoteImageSrc(base, url);
}
