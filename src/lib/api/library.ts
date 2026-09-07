import type { BlacklistEntry, GalleryInfo, HistoryEntry, QuickSearch } from "../types";

import { invokeOrThrow, isTauri, loadPreferences } from "./transport";

export async function getHistory(page: number): Promise<HistoryEntry[]> {
  if (isTauri()) return invokeOrThrow("get_history", { page });
  return (await loadPreferences()).getHistory(page);
}

export async function recordHistory(gallery: GalleryInfo): Promise<void> {
  if (isTauri()) return;
  return (await loadPreferences()).recordHistory(gallery);
}

export async function removeHistory(gid: number): Promise<void> {
  if (isTauri()) return invokeOrThrow("delete_history", { gid });
  return (await loadPreferences()).removeHistory(gid);
}

export async function clearHistory(): Promise<void> {
  if (isTauri()) return invokeOrThrow("clear_history");
  return (await loadPreferences()).clearHistory();
}

export async function quickSearchList(): Promise<QuickSearch[]> {
  if (isTauri()) return invokeOrThrow("get_quick_searches");
  return (await loadPreferences()).quickSearchList();
}

export async function createQuickSearch(entry: {
  name: string;
  keyword: string;
  categories?: string | null;
  minRating?: number | null;
  minPages?: number | null;
  maxPages?: number | null;
}): Promise<QuickSearch> {
  if (isTauri()) {
    return invokeOrThrow("add_quick_search", {
      name: entry.name,
      keyword: entry.keyword,
      categories: entry.categories ?? null,
      minRating: entry.minRating ?? null,
      minPages: entry.minPages ?? null,
      maxPages: entry.maxPages ?? null,
    });
  }
  return (await loadPreferences()).createQuickSearch(entry);
}

export async function deleteQuickSearch(id: number): Promise<void> {
  if (isTauri()) return invokeOrThrow("delete_quick_search", { id });
  return (await loadPreferences()).deleteQuickSearch(id);
}

export async function blacklistList(): Promise<BlacklistEntry[]> {
  if (isTauri()) return invokeOrThrow("get_blacklist");
  return (await loadPreferences()).blacklistList();
}

export async function blacklistCreate(type: string, content: string): Promise<BlacklistEntry> {
  if (isTauri()) return invokeOrThrow("add_blacklist", { kind: type, content });
  return (await loadPreferences()).blacklistCreate(type, content);
}

export async function blacklistDelete(id: number): Promise<void> {
  if (isTauri()) return invokeOrThrow("delete_blacklist", { id });
  return (await loadPreferences()).blacklistDelete(id);
}
