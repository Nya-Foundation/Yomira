import type { ListMode } from "./api/galleries";

/**
 * What each browse source's backend actually honours. Popular is a single
 * fixed feed and Toplist takes only its timespan and page, so `ListUrlBuilder`
 * discards keyword, categories and advanced search for both. Offering those
 * controls there shows a filter that silently does nothing.
 *
 * This is the one source of truth: the list toolbar and the header's search
 * affordance both read it, so a mode cannot end up with neither.
 */
export const MODE_CAPS: Record<ListMode, { query: boolean; jump: boolean }> = {
  home: { query: true, jump: true },
  watched: { query: true, jump: true },
  toplist: { query: false, jump: true },
  popular: { query: false, jump: false },
};

const MODE_BY_PATH: Record<string, ListMode> = {
  "/": "home",
  "/popular": "popular",
  "/watched": "watched",
  "/toplist": "toplist",
};

/** Does this route render its own search field? */
export function hasOwnSearch(pathname: string): boolean {
  const mode = MODE_BY_PATH[pathname];
  return mode !== undefined && MODE_CAPS[mode].query;
}
