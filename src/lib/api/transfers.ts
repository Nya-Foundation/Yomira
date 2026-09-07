import type { ArchiveResolution, ArchiverInfo, TorrentInfo } from "../types";

import { invokeOrThrow, isTauri, loadMock } from "./transport";

// ---------------------------------------------------------------------------
// Torrents and archiver. All of these require a signed-in session on the
// backend; `downloadTorrent` / `downloadArchive` resolve to the saved file
// path on disk.
// ---------------------------------------------------------------------------

export async function getTorrentList(gid: number, token: string): Promise<TorrentInfo[]> {
  if (!isTauri()) return (await loadMock()).mock.getTorrentList(gid);
  return invokeOrThrow("get_torrent_list", { gid, token });
}

export async function downloadTorrent(gid: number, token: string, url: string): Promise<string> {
  if (!isTauri()) return (await loadMock()).mock.downloadTorrent(gid, url);
  return invokeOrThrow("download_torrent", { gid, token, url });
}

export async function getArchiverInfo(gid: number, token: string): Promise<ArchiverInfo> {
  if (!isTauri()) return (await loadMock()).mock.getArchiverInfo(gid);
  return invokeOrThrow("get_archiver_info", { gid, token });
}

export async function downloadArchive(
  gid: number,
  token: string,
  resolution: ArchiveResolution,
): Promise<string> {
  if (!isTauri()) return (await loadMock()).mock.downloadArchive(gid, resolution);
  return invokeOrThrow("download_archive", { gid, token, resolution });
}
