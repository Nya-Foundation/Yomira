import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Archive, Download } from "lucide-react";
import {
  downloadArchive,
  downloadTorrent,
  getArchiverInfo,
  getTorrentList,
} from "../../lib/api";
import type { ArchiveResolution } from "../../lib/types";
import { useT } from "../../lib/i18n";
import { formatBytes } from "../../lib/utils";
import { useToastedMutation } from "../../lib/use-toasted-mutation";
import { Button } from "../ui/button";
import { Dialog } from "../ui/dialog";
import { Spinner } from "../ui/spinner";
import { Tooltip } from "../ui/tooltip";

/**
 * Torrent / archiver download actions. Both require a signed-in session on
 * the backend; when signed out the buttons stay enabled-looking but disabled,
 * with a sign-in tooltip.
 */
export function DownloadOptionsSection({
  gid,
  token,
  torrentCount,
  signedIn,
}: {
  gid: number;
  token: string;
  torrentCount: number;
  signedIn: boolean;
}) {
  const t = useT();
  const [torrentsOpen, setTorrentsOpen] = useState(false);
  const [archiverOpen, setArchiverOpen] = useState(false);

  const torrentsQuery = useQuery({
    queryKey: ["torrents", gid],
    queryFn: () => getTorrentList(gid, token),
    enabled: torrentsOpen,
  });
  const archiverQuery = useQuery({
    queryKey: ["archiver", gid],
    queryFn: () => getArchiverInfo(gid, token),
    enabled: archiverOpen,
  });

  const torrentMutation = useToastedMutation({
    mutationFn: (url: string) => downloadTorrent(gid, token, url),
    successMessage: t("detail.torrentDownloaded"),
    successDescription: (path) => path,
    errorMessage: t("detail.torrentFailed"),
  });
  const archiveMutation = useToastedMutation({
    mutationFn: (resolution: ArchiveResolution) => downloadArchive(gid, token, resolution),
    successMessage: t("detail.archiveSaved"),
    successDescription: (path) => path,
    errorMessage: t("detail.archiveFailed"),
  });

  return (
    <section className="mt-8 flex flex-wrap gap-2" aria-label={t("detail.downloadOptions")}>
      <Tooltip content={signedIn ? t("detail.torrents", { n: torrentCount }) : t("detail.downloadsNeedSignIn")}>
        <span tabIndex={0} className="inline-flex">
          <Button
            variant="outline"
            disabled={!signedIn}
            onClick={() => setTorrentsOpen(true)}
            className="gap-1.5"
          >
            <Download size={14} strokeWidth={1.75} /> {t("detail.torrents", { n: torrentCount })}
          </Button>
        </span>
      </Tooltip>
      <Tooltip content={signedIn ? t("detail.archiver") : t("detail.downloadsNeedSignIn")}>
        <span tabIndex={0} className="inline-flex">
          <Button
            variant="outline"
            disabled={!signedIn}
            onClick={() => setArchiverOpen(true)}
            className="gap-1.5"
          >
            <Archive size={14} strokeWidth={1.75} /> {t("detail.archiver")}
          </Button>
        </span>
      </Tooltip>

      <Dialog open={torrentsOpen} onOpenChange={setTorrentsOpen} title={t("detail.torrentsTitle")}>
        {torrentsQuery.isLoading ? (
          <div className="flex justify-center py-6">
            <Spinner />
          </div>
        ) : torrentsQuery.isError ? (
          <p className="py-2 text-small text-danger">{t("detail.torrentFailed")}</p>
        ) : (torrentsQuery.data?.length ?? 0) === 0 ? (
          <p className="py-2 text-small text-muted-foreground">{t("detail.torrentEmpty")}</p>
        ) : (
          <ul className="space-y-2">
            {torrentsQuery.data!.map((tor) => (
              <li
                key={tor.url}
                className="flex items-center gap-3 rounded-lg border border-border bg-surface p-3"
              >
                <div className="min-w-0 flex-1">
                  <p className="truncate text-base font-medium">{tor.name}</p>
                  <p className="text-caption text-faint-foreground">
                    {t("detail.torrentMeta", {
                      size: formatBytes(tor.size),
                      seeds: tor.seeds,
                      peers: tor.peers,
                      downloads: tor.downloads,
                    })}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={torrentMutation.isPending}
                  onClick={() => torrentMutation.mutate(tor.url)}
                  aria-label={`${t("detail.torrentDownload")} ${tor.name}`}
                >
                  {torrentMutation.isPending && torrentMutation.variables === tor.url ? (
                    <Spinner size={13} />
                  ) : (
                    <Download size={13} strokeWidth={1.75} />
                  )}
                  {t("detail.torrentDownload")}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Dialog>

      <Dialog open={archiverOpen} onOpenChange={setArchiverOpen} title={t("detail.archiverTitle")}>
        {archiverQuery.isLoading ? (
          <div className="flex justify-center py-6">
            <Spinner />
          </div>
        ) : archiverQuery.isError ? (
          <p className="py-2 text-small text-danger">{t("detail.archiveFailed")}</p>
        ) : (
          <>
            {archiverQuery.data && (
              <p className="mb-4 text-base leading-6 text-muted-foreground">
                {archiverQuery.data.costText}
              </p>
            )}
            <div className="flex flex-wrap justify-end gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={archiveMutation.isPending}
                onClick={() => archiveMutation.mutate("res")}
              >
                {archiveMutation.isPending && archiveMutation.variables === "res" && (
                  <Spinner size={13} />
                )}
                {t("detail.downloadRes")}
              </Button>
              <Button
                size="sm"
                disabled={archiveMutation.isPending}
                onClick={() => archiveMutation.mutate("org")}
              >
                {archiveMutation.isPending && archiveMutation.variables === "org" && (
                  <Spinner size={13} />
                )}
                {t("detail.downloadOrg")}
              </Button>
            </div>
          </>
        )}
      </Dialog>
    </section>
  );
}
