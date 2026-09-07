import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Inbox, LayoutGrid, List } from "lucide-react";
import type { GalleryInfo } from "../../lib/types";
import { GalleryCard, GalleryCardSkeleton } from "./gallery-card";
import { EmptyState } from "../ui/empty-state";
import { ErrorState } from "../ui/error-state";
import { Spinner } from "../ui/spinner";
import { useT } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { Button } from "../ui/button";
import { cn } from "../../lib/utils";

const PAD_X = 16;

interface GalleryGridProps {
  galleries: GalleryInfo[];
  loading: boolean;
  error: unknown;
  onRetry: () => void;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
  emptyTitle?: string;
  emptyDescription?: string;
  emptyAction?: ReactNode;
  className?: string;
  /**
   * When set, the scroll offset is persisted to sessionStorage under this key
   * (throttled via rAF) and restored on remount once data is present, so
   * back-navigation returns the user to where they were.
   */
  scrollKey?: string;
}

export function GalleryGrid({
  galleries,
  loading,
  error,
  onRetry,
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
  emptyTitle,
  emptyDescription,
  emptyAction,
  className,
  scrollKey,
}: GalleryGridProps) {
  const t = useT();
  const view = useSettings((s) => s.settings.galleryView);
  const updateSettings = useSettings((s) => s.update);
  const scale = useSettings((s) => s.settings.displayScale);
  const density = useSettings((s) => s.settings.density);
  const gap = density === "compact" ? 12 : 20;
  const baseCol = density === "compact" ? 150 : 180;
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const restoredKeyRef = useRef<string | null>(null);
  const saveRafRef = useRef<number | null>(null);
  const onLoadMoreRef = useRef(onLoadMore);
  onLoadMoreRef.current = onLoadMore;

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      setWidth(entries[0]?.contentRect.width ?? 0);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [loading, galleries.length === 0]);

  const storageKey = scrollKey ? `exviewer.scroll.${scrollKey}` : null;

  const saveScroll = useCallback(() => {
    if (!storageKey) return;
    const el = scrollRef.current;
    if (el) sessionStorage.setItem(storageKey, String(el.scrollTop));
  }, [storageKey]);

  // restore the saved offset before paint as soon as cached data is present
  useLayoutEffect(() => {
    if (!scrollKey || restoredKeyRef.current === scrollKey) return;
    const el = scrollRef.current;
    if (!el) return;
    // wait for rows to exist so the scrollable height covers the offset
    if (galleries.length === 0 || width === 0) return;
    restoredKeyRef.current = scrollKey;
    if (storageKey) {
      const raw = sessionStorage.getItem(storageKey);
      const top = raw == null ? 0 : Number(raw);
      el.scrollTop = Number.isFinite(top) ? Math.max(0, top) : 0;
    }
  }, [scrollKey, storageKey, galleries.length, width]);

  // flush any pending throttled save on unmount so the final offset survives
  useEffect(() => {
    return () => {
      if (saveRafRef.current != null) cancelAnimationFrame(saveRafRef.current);
      saveScroll();
    };
  }, [saveScroll]);

  const onScroll = useCallback(() => {
    if (saveRafRef.current != null) return;
    saveRafRef.current = requestAnimationFrame(() => {
      saveRafRef.current = null;
      saveScroll();
      const el = scrollRef.current;
      if (el && hasNextPage && !isFetchingNextPage && !error && el.scrollHeight - el.scrollTop - el.clientHeight < 600) {
        onLoadMoreRef.current();
      }
    });
  }, [saveScroll, hasNextPage, isFetchingNextPage, error]);

  const minCol = (width < 480 ? 140 : baseCol) * scale;
  const cols = view === "detail" ? 1 : Math.max(1, Math.floor((width - PAD_X * 2 + gap) / (minCol + gap)));
  const colW = width > 0 ? (width - PAD_X * 2 - gap * (cols - 1)) / cols : minCol;
  const rowH = view === "detail" ? 170 * scale + gap : colW * (4 / 3) + 130 * scale + gap;
  const rows = Math.ceil(galleries.length / cols);

  const virtualizer = useVirtualizer({
    count: rows,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowH,
    overscan: 2,
  });

  useLayoutEffect(() => {
    virtualizer.measure();
  }, [width, scale, density, view, virtualizer]);

  // load-more sentinel: created once per open/fetching state; the callback is
  // read through a ref so loaded pages don't churn the observer
  useEffect(() => {
    const el = scrollRef.current;
    if (!el || !hasNextPage || isFetchingNextPage || error) return;
    const sentinel = el.querySelector("[data-load-more]");
    if (!sentinel) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) onLoadMoreRef.current();
      },
      { root: el, rootMargin: "600px" },
    );
    io.observe(sentinel);
    return () => io.disconnect();
  }, [hasNextPage, isFetchingNextPage, error, loading, galleries.length, view]);

  if (error && !loading && galleries.length === 0) {
    return (
      <div className={cn("flex overflow-y-auto p-4", className)}>
        <ErrorState description={t("list.errorDesc")} onRetry={onRetry} />
      </div>
    );
  }

  if (!loading && galleries.length === 0 && !hasNextPage) {
    return (
      <div className={cn("flex overflow-y-auto p-4", className)}>
        <EmptyState
          icon={Inbox}
          title={emptyTitle ?? t("list.emptyDefault")}
          description={emptyDescription}
          action={emptyAction}
        />
      </div>
    );
  }

  return (
    <div className={cn("flex min-h-0 flex-col", className)}>
      <div className="flex shrink-0 items-center justify-end gap-1 border-b border-border px-4 py-1.5" role="group" aria-label={t("list.galleryView")}>
        {(["thumbnail", "detail"] as const).map((mode) => (
          <Button key={mode} variant={view === mode ? "subtle" : "ghost"} size="sm"
            aria-pressed={view === mode} onClick={() => {
              if (scrollRef.current) scrollRef.current.scrollTop = 0;
              updateSettings({ galleryView: mode });
            }} className="h-10 gap-1.5">
            {mode === "thumbnail" ? <LayoutGrid size={15} /> : <List size={15} />}
            {t(mode === "thumbnail" ? "list.thumbnailView" : "list.detailView")}
          </Button>
        ))}
      </div>
    <div ref={scrollRef} onScroll={onScroll} className="min-h-0 flex-1 overflow-y-auto">
      {width > 0 && (
        <div
          className="relative mt-4"
          style={{ height: `${virtualizer.getTotalSize()}px` }}
        >
          {virtualizer.getVirtualItems().map((row) => {
            const start = row.index * cols;
            const slice = galleries.slice(start, start + cols);
            return (
              <div
                key={row.key}
                data-index={row.index}
                ref={virtualizer.measureElement}
                className="absolute left-0 top-0 grid"
                style={{
                  transform: `translateY(${row.start}px)`,
                  width,
                  padding: `0 ${PAD_X}px ${gap}px`,
                  gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`,
                  gap,
                }}
              >
                {slice.map((g) => (
                  <div key={g.gid} className="min-w-0">
                    <GalleryCard gallery={g} detail={view === "detail"} />
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      )}

      {loading && (
        <div
          className="grid gap-4 p-4"
          style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
        >
          {Array.from({ length: cols * 2 }, (_, i) => (
            <GalleryCardSkeleton key={i} />
          ))}
        </div>
      )}

      {!!error && galleries.length > 0 && <div className="flex justify-center p-4"><Button variant="outline" onClick={onRetry}>{t("common.retry")}</Button></div>}
      <div data-load-more className="flex h-14 items-center justify-center">
        {hasNextPage && !loading && !error && (
          <Button variant="ghost" size="sm" onClick={onLoadMore} disabled={isFetchingNextPage} className="gap-2">
            {isFetchingNextPage && <Spinner size={18} />}
            {t(isFetchingNextPage ? "list.loadingMore" : "list.loadMore")}
          </Button>
        )}
        {!hasNextPage && !loading && galleries.length > 0 && (
          <span className="text-caption text-faint-foreground">{t("list.endOfResults")}</span>
        )}
      </div>
    </div>
    </div>
  );
}
