import type {
  Comment,
  DownloadedGallery,
  FavoriteItem,
  GalleryDetail,
  GalleryListResponse,
  ListParams,
  ListPage,
  PageData,
  PreviewSet,
  ToplistTimespan
} from "../types";

import { invokeOrThrow, isTauri, loadMock } from "./transport";

export type ListMode = "home" | "popular" | "watched" | "toplist";

export async function getGalleryList(
  params: Omit<ListParams, "page"> & { page: ListPage },
  mode: ListMode = "home",
  timespan: ToplistTimespan = "month",
): Promise<GalleryListResponse> {
  const request = { ...params, page: typeof params.page === "number" ? params.page : 0,
    cursor: typeof params.page === "string" ? params.page : undefined };
  if (!isTauri()) return (await loadMock()).mock.getGalleryList(mode, request, timespan);
  return invokeOrThrow("get_gallery_list", { params: request, mode, timespan });
}

export async function getGalleryDetail(gid: number, token: string): Promise<GalleryDetail> {
  if (!isTauri()) return (await loadMock()).mock.getGalleryDetail(gid);
  return invokeOrThrow("get_gallery_detail", { gid, token });
}

export async function getPreviewSet(gid: number, token: string, index: number): Promise<PreviewSet> {
  if (!isTauri()) return (await loadMock()).mock.getPreviewSet(gid, index);
  return invokeOrThrow("get_preview_set", { gid, token, index });
}

export async function getDownloadedGallery(gid: number, token: string): Promise<DownloadedGallery | null> {
  if (!isTauri()) return (await loadMock()).mock.getDownloadedGallery(gid, token);
  return invokeOrThrow("get_downloaded_gallery", { gid, token });
}

export async function getPageData(
  gid: number,
  token: string,
  index: number,
  localOnly = false,
): Promise<PageData> {
  if (localOnly) {
    if (!isTauri()) return (await loadMock()).mock.getDownloadedPage(gid, token, index);
    return invokeOrThrow("get_downloaded_page", { gid, token, index });
  }
  if (!isTauri()) return (await loadMock()).mock.getPageData(gid, index);
  return invokeOrThrow("get_page_data", { gid, token, index });
}

export type FavoriteSort = "f" | "p";

export async function getFavorites(
  page: ListPage,
  slot: number | null,
  sort?: FavoriteSort | null,
): Promise<{ items: FavoriteItem[]; nextPage: ListPage | null }> {
  if (!isTauri()) return (await loadMock()).mock.getFavorites(typeof page === "number" ? page : 0, slot, sort ?? null);
  const args: Record<string, unknown> = { page: typeof page === "number" ? page : 0, cursor: typeof page === "string" ? page : undefined, slot };
  if (sort) args.sort = sort;
  return invokeOrThrow("get_favorites", args);
}

/**
 * The favorite note for one gallery. The backend only exposes notes inside
 * paged favorites responses, so in the app this walks pages until it finds
 * the entry (react-query caches the result per gallery).
 */
export async function getFavoriteNote(gid: number): Promise<string> {
  if (!isTauri()) return (await loadMock()).mock.getFavoriteNote(gid);
  let page: ListPage = 0;
  const seen = new Set<ListPage>();
  while (!seen.has(page)) {
    seen.add(page);
    const res = await getFavorites(page, null);
    const hit = res.items.find((i) => i.gid === gid);
    if (hit) return hit.favoriteNote ?? "";
    if (res.nextPage == null) return "";
    page = res.nextPage;
  }
  return "";
}

export async function modifyFavoriteNote(gid: number, token: string, note: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.modifyFavoriteNote(gid, note);
  return invokeOrThrow("modify_favorite_note", { gid, token, note });
}

export async function getComments(gid: number, token: string, page: number): Promise<Comment[]> {
  if (!isTauri()) return (await loadMock()).mock.getComments(gid, page);
  return invokeOrThrow("get_comments", { gid, token, page });
}

export async function getImage(url: string): Promise<string> {
  if (!isTauri()) return (await loadMock()).mock.getImage(url);
  return invokeOrThrow("get_image", { url });
}

export async function toggleFavorite(
  gid: number,
  token: string,
  slot: number,
): Promise<{ isFavorited: boolean; slot: number }> {
  if (!isTauri()) return (await loadMock()).mock.toggleFavorite(gid, slot);
  return invokeOrThrow("toggle_favorite", { gid, token, slot });
}

export async function rateGallery(
  gid: number,
  token: string,
  rating: number,
): Promise<{ rating: number }> {
  if (!isTauri()) return (await loadMock()).mock.rateGallery(gid, rating);
  return invokeOrThrow("rate_gallery", { gid, token, rating });
}
