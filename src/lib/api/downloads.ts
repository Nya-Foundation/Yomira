import type { DownloadEntry } from "../types";

import { invokeOrThrow, isTauri, loadMock } from "./transport";

// ---------------------------------------------------------------------------
// Downloads
//
// Any change on the Rust side fires the `download://changed` Tauri event;
// DownloadsPage listens for it (guarded by isTauri) and invalidates the
// `["downloads"]` query instead of polling. In the browser demo the
// localStorage mock answers and mutations invalidate synchronously.
// ---------------------------------------------------------------------------

export const DOWNLOAD_CHANGED_EVENT = "download://changed";

export async function listDownloads(): Promise<DownloadEntry[]> {
  if (!isTauri()) return (await loadMock()).mock.listDownloads();
  return invokeOrThrow("list_downloads");
}

export async function addDownload(gid: number, token: string, label?: string | null): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.addDownload(gid, token, label ?? null);
  const args: Record<string, unknown> = { gid, token };
  if (label) args.label = label;
  return invokeOrThrow("add_download", args);
}

export async function removeDownload(gid: number, deleteFiles: boolean): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.removeDownload(gid, deleteFiles);
  return invokeOrThrow("remove_download", { gid, deleteFiles });
}

export async function startDownload(gid: number): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.startDownload(gid);
  return invokeOrThrow("start_download", { gid });
}

export async function pauseDownload(gid: number): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.pauseDownload(gid);
  return invokeOrThrow("pause_download", { gid });
}

export async function downloadLabels(): Promise<string[]> {
  if (!isTauri()) return (await loadMock()).mock.downloadLabels();
  return invokeOrThrow("download_labels");
}

export async function addDownloadLabel(name: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.addDownloadLabel(name);
  return invokeOrThrow("add_download_label", { name });
}

export async function deleteDownloadLabel(name: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.deleteDownloadLabel(name);
  return invokeOrThrow("delete_download_label", { name });
}

export async function setDownloadLabel(gid: number, label?: string | null): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.setDownloadLabel(gid, label ?? null);
  return invokeOrThrow("set_download_label", { gid, label: label ?? null });
}

export async function getDownloadRoot(): Promise<string> {
  if (!isTauri()) return (await loadMock()).mock.getDownloadRoot();
  return invokeOrThrow("get_download_root");
}

export async function setDownloadRoot(path: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.setDownloadRoot(path);
  return invokeOrThrow("set_download_root", { path });
}

// ---------------------------------------------------------------------------
// Extras: image search, news, quota, search history
// ---------------------------------------------------------------------------
