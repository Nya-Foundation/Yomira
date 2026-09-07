import type { GalleryListResponse, NewsItem, Quota } from "../types";

import { invokeOrThrow, isTauri, loadMock } from "./transport";

export async function imageLookup(
  dataBase64: string,
  filename: string,
  similar: boolean,
  coversOnly: boolean,
  expunged: boolean,
): Promise<GalleryListResponse> {
  if (!isTauri()) return (await loadMock()).mock.imageLookup();
  return invokeOrThrow("image_lookup", { dataBase64, filename, similar, coversOnly, expunged });
}

export async function getNews(page: number): Promise<NewsItem[]> {
  if (!isTauri()) return (await loadMock()).mock.getNews(page);
  return invokeOrThrow("get_news", { page });
}

export async function getQuota(): Promise<Quota> {
  if (!isTauri()) return (await loadMock()).mock.getQuota();
  return invokeOrThrow("get_quota");
}

export async function searchHistory(): Promise<string[]> {
  if (!isTauri()) return (await loadMock()).mock.searchHistory();
  return invokeOrThrow("search_history");
}

export async function recordSearch(term: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.recordSearch(term);
  return invokeOrThrow("record_search", { term });
}

export async function clearSearchHistory(): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.clearSearchHistory();
  return invokeOrThrow("clear_search_history");
}

export async function deleteSearchTerm(term: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.deleteSearchTerm(term);
  return invokeOrThrow("delete_search_term", { term });
}
