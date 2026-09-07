import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useContext, useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { useAppNavigation } from "../lib/app-navigation";
import { ContinuousReader } from "../components/reader/continuous-reader";
import { OfflineContext } from "../components/reader/page-query";
import { PagedReader } from "../components/reader/paged-reader";
import { ReaderToolbar } from "../components/reader/toolbar";
import { Button } from "../components/ui/button";
import { Spinner } from "../components/ui/spinner";
import { getDownloadedGallery, getGalleryDetail } from "../lib/api";
import { useT } from "../lib/i18n";
import { readProgress, saveProgress } from "../lib/reading-progress";
import { useSettings } from "../lib/settings";
import type { DownloadedGallery, GalleryDetail } from "../lib/types";
import { useDownloadEvents } from "../lib/use-download-events";
import { isEditableTarget } from "../lib/utils";

export function ReaderPage() {
  const { gid, token } = useParams();
  const [params] = useSearchParams();
  const offline = params.get("source") === "download";
  return (
    <OfflineContext.Provider value={offline}>
      <ReaderSession key={`${gid}/${token}/${offline}`} />
    </OfflineContext.Provider>
  );
}

function ReaderSession() {
  const { gid = "0", token = "" } = useParams();
  const gidNum = Number(gid);
  const { back } = useAppNavigation();
  const offline = useContext(OfflineContext);
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const readerDirection = useSettings((s) => s.settings.readerDirection);
  const readerMode = useSettings((s) => s.settings.readerMode);
  const updateSettings = useSettings((s) => s.update);
  const t = useT();
  const [controlsVisible, setControlsVisible] = useState(true);

  const detailQuery = useQuery<DownloadedGallery | GalleryDetail | null>({
    queryKey: [offline ? "downloaded-gallery" : "gallery-detail", gidNum, token],
    queryFn: () => offline ? getDownloadedGallery(gidNum, token) : getGalleryDetail(gidNum, token),
    retry: false,
    staleTime: offline ? 0 : Infinity,
    refetchOnWindowFocus: offline,
  });
  useDownloadEvents(() => {
    void queryClient.invalidateQueries({ queryKey: ["downloaded-gallery", gidNum, token] });
  }, offline);
  const pageCount = detailQuery.data?.pages ?? 0;

  const [index, setIndex] = useState(() => {
    const fromUrl = searchParams.get("p");
    const page = Number(fromUrl);
    return fromUrl !== null && Number.isInteger(page) && page >= 0
      ? Math.min(page, 10_000)
      : readProgress(gidNum);
  });

  // clamp once detail is known
  useEffect(() => {
    if (pageCount > 0) setIndex((i) => Math.min(i, pageCount - 1));
  }, [pageCount]);

  // persist position
  useEffect(() => {
    if (pageCount > 0 && index < pageCount) saveProgress(gidNum, index);
  }, [gidNum, index, pageCount]);

  const go = useCallback(
    (delta: number) => {
      setIndex((i) => Math.max(0, Math.min(i + delta, pageCount - 1 || 0)));
    },
    [pageCount],
  );

  // keyboard nav
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (document.querySelector("dialog[open]") || isEditableTarget(e.target) || e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.key === " " && e.target instanceof Element && e.target.closest("button, a")) return;
      if (e.key === "ArrowRight" || e.key === " " || e.key.toLowerCase() === "j") {
        e.preventDefault();
        go(e.key === "ArrowRight" && readerDirection === "rtl" ? -1 : 1);
      } else if (e.key === "ArrowLeft" || e.key.toLowerCase() === "k") {
        e.preventDefault();
        go(e.key === "ArrowLeft" && readerDirection === "rtl" ? 1 : -1);
      } else if (e.key === "PageDown") {
        e.preventDefault();
        go(1);
      } else if (e.key === "PageUp") {
        e.preventDefault();
        go(-1);
      } else if (e.key === "Home") {
        e.preventDefault();
        setIndex(0);
      } else if (e.key === "End") {
        e.preventDefault();
        setIndex(Math.max(0, pageCount - 1));
      } else if (e.key === "Escape") {
        back();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go, pageCount, back, readerDirection]);

  if (detailQuery.isLoading) {
    return (
      <div className="flex h-full items-center justify-center bg-background">
        <Spinner size={24} />
      </div>
    );
  }
  if (detailQuery.isError || pageCount === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 bg-background px-6 text-center">
        <p className="text-small text-muted-foreground">{t(offline ? "reader.offlineUnavailable" : "reader.loadFailed")}</p>
        <Button variant="outline" onClick={() => void detailQuery.refetch()}>{t("common.retry")}</Button>
        <Link to={offline ? "/downloads" : "/"} className="text-small font-medium text-accent hover:underline">
          {t(offline ? "reader.backDownloads" : "reader.backHome")}
        </Link>
      </div>
    );
  }

  return (
    <div className="relative flex h-full flex-col">
      <ReaderToolbar
        hidden={!controlsVisible}
        setHidden={(hidden) => setControlsVisible(!hidden)}
        index={index}
        pageCount={pageCount}
        mode={readerMode}
        onModeChange={(m) => updateSettings({ readerMode: m })}
        onIndexChange={(i) => setIndex(Math.max(0, Math.min(i, pageCount - 1)))}
        title={detailQuery.data?.title ?? ""}
        offlineStatus={offline && detailQuery.data && "availablePages" in detailQuery.data ? t("reader.offlineStatus", { a: detailQuery.data.availablePages.length, b: pageCount }) : undefined}
      />
      {readerMode === "paged" ? (
        <PagedReader controlsVisible={controlsVisible} gid={gidNum} token={token} index={index} pageCount={pageCount} go={go} />
      ) : (
        <ContinuousReader
          controlsVisible={controlsVisible}
          gid={gidNum}
          token={token}
          index={index}
          pageCount={pageCount}
          onIndexChange={setIndex}
        />
      )}
    </div>
  );
}
