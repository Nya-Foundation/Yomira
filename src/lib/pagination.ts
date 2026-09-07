import type { ListPage } from "./types";

/** Stop repeated cursors instead of fetching the same page indefinitely. */
export function nextListPage(next: ListPage | null, loaded: ListPage[]): ListPage | undefined {
  return next != null && !loaded.includes(next) ? next : undefined;
}
