import { useQueryClient } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight, RefreshCw, ZoomIn } from "lucide-react";
import { useCallback, useContext, useEffect, useRef, useState } from "react";
import { useT } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { cn } from "../../lib/utils";
import { steppedZoom, useReaderZoom, type ZoomAction } from "../../lib/use-zoom-shortcuts";
import { Spinner } from "../ui/spinner";

import { PageErrorCard, ReaderImage } from "./page-image";
import { OfflineContext, pageQueryOptions, usePageQuery } from "./page-query";

export function PagedReader({
  controlsVisible,
  gid,
  token,
  index,
  pageCount,
  go,
}: {
  controlsVisible: boolean;
  gid: number;
  token: string;
  index: number;
  pageCount: number;
  go: (delta: number) => void;
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const localOnly = useContext(OfflineContext);
  const readerPreloadCount = useSettings((s) => s.settings.readerPreloadCount);
  const direction = useSettings((s) => s.settings.readerDirection);
  const gesture = useRef<{ x: number; y: number; blocked: boolean } | null>(null);
  const suppressClickUntil = useRef(0);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const pinchDist = useRef<number | null>(null);
  const wheelAccum = useRef(0);

  useEffect(() => {
    wheelAccum.current = 0;
    pointers.current.clear();
    gesture.current = null;
  }, [index]);

  const { query: pageQuery, retry } = usePageQuery(gid, token, index, true);

  // lookahead: preload the configured number of pages ahead
  const preloadCount = Math.max(0, Math.min(10, Math.round(readerPreloadCount)));
  useEffect(() => {
    for (let ahead = index + 1; ahead <= index + preloadCount; ahead++) {
      if (ahead < pageCount) {
        void queryClient.prefetchQuery(pageQueryOptions(gid, token, ahead, localOnly));
      }
    }
  }, [gid, token, index, pageCount, preloadCount, queryClient, localOnly]);

  useEffect(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, [index]);

  const applyZoom = useCallback(
    (next: number) => {
      const clamped = Math.max(0.25, Math.min(next, 4));
      setZoom(clamped);
      if (clamped <= 1) setPan({ x: 0, y: 0 });
    },
    [],
  );

  useReaderZoom(useCallback((action: ZoomAction) => applyZoom(steppedZoom(zoom, action, 0.25, 4)), [zoom, applyZoom]));

  const onWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) return; // Handled by the non-passive global listener.
      // Paged mode: plain wheel turns pages (accumulate small/trackpad
      // deltas); when zoomed in it pans vertically instead.
      if (zoom > 1) {
        setPan((p) => ({ x: p.x, y: p.y - e.deltaY }));
        return;
      }
      wheelAccum.current += e.deltaY;
      if (wheelAccum.current >= 80) {
        wheelAccum.current = 0;
        go(1);
      } else if (wheelAccum.current <= -80) {
        wheelAccum.current = 0;
        go(-1);
      }
    },
    [zoom, go],
  );

  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    suppressClickUntil.current = 0;
    const target = e.target as Element;
    if (target.closest("button:not([data-reader-edge]), a, input")) return;
    target.setPointerCapture?.(e.pointerId);
    if (pointers.current.size === 0) gesture.current = { x: e.clientX, y: e.clientY, blocked: zoom > 1 };
    pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.current.size === 2) {
      if (gesture.current) gesture.current.blocked = true;
      const [a, b] = [...pointers.current.values()];
      pinchDist.current = Math.hypot(a.x - b.x, a.y - b.y);
    }
  };

  const onPointerMove = (e: React.PointerEvent) => {
    const prev = pointers.current.get(e.pointerId);
    if (!prev) return;
    pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.current.size === 2 && pinchDist.current != null) {
      const [a, b] = [...pointers.current.values()];
      const dist = Math.hypot(a.x - b.x, a.y - b.y);
      applyZoom(zoom * (dist / pinchDist.current));
      pinchDist.current = dist;
    } else if (pointers.current.size === 1 && zoom > 1) {
      setPan((p) => ({ x: p.x + e.clientX - prev.x, y: p.y + e.clientY - prev.y }));
    }
  };

  const onPointerUp = (e: React.PointerEvent) => {
    if (!pointers.current.has(e.pointerId)) return;
    const start = gesture.current;
    pointers.current.delete(e.pointerId);
    if (pointers.current.size < 2) pinchDist.current = null;
    if (pointers.current.size || !start) return;
    gesture.current = null;
    const dx = e.clientX - start.x;
    const dy = e.clientY - start.y;
    if (start.blocked || Math.hypot(dx, dy) > 12) suppressClickUntil.current = Date.now() + 400;
    if (e.type === "pointercancel" || start.blocked) return;
    if (Math.abs(dx) >= 45 && Math.abs(dx) > Math.abs(dy) * 1.3) {
      go((dx < 0 ? 1 : -1) * (direction === "rtl" ? -1 : 1));
    }
  };

  const error = pageQuery.error;

  return (
    <div
      className={cn("relative flex min-h-0 flex-1 items-center justify-center overflow-hidden touch-none bg-background", controlsVisible && "pt-24")}
      onWheel={onWheel}
      data-reader-surface
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onClickCapture={(e) => {
        if (Date.now() < suppressClickUntil.current) { e.preventDefault(); e.stopPropagation(); }
      }}
    >
      {/* Edge navigation yields to drag and pinch while zoomed. */}
      <button
        data-reader-edge
        aria-label={t("reader.prevPage")}
        onClick={() => go(-1)}
        disabled={index === 0 || zoom > 1}
        className={`absolute inset-y-0 ${direction === "rtl" ? "right-0" : "left-0"} z-20 w-[15%] disabled:pointer-events-none cursor-w-resize disabled:cursor-default`}
      />
      <button
        data-reader-edge
        aria-label={t("reader.nextPage")}
        onClick={() => go(1)}
        disabled={index >= pageCount - 1 || zoom > 1}
        className={`absolute inset-y-0 ${direction === "rtl" ? "left-0" : "right-0"} z-20 w-[15%] disabled:pointer-events-none cursor-e-resize disabled:cursor-default`}
      />

      {/* hover arrows */}
      {index > 0 && (
        <div className={cn("pointer-events-none absolute top-1/2 z-10 -translate-y-1/2 rounded-full border border-border bg-elevated/80 p-2 text-muted-foreground", direction === "rtl" ? "right-3" : "left-3")}>
          {direction === "rtl" ? <ChevronRight size={18} strokeWidth={1.75} /> : <ChevronLeft size={18} strokeWidth={1.75} />}
        </div>
      )}
      {index < pageCount - 1 && (
        <div className={cn("pointer-events-none absolute top-1/2 z-10 -translate-y-1/2 rounded-full border border-border bg-elevated/80 p-2 text-muted-foreground", direction === "rtl" ? "left-3" : "right-3")}>
          {direction === "rtl" ? <ChevronLeft size={18} strokeWidth={1.75} /> : <ChevronRight size={18} strokeWidth={1.75} />}
        </div>
      )}

      {pageQuery.isPending && !error && (
        <div className="flex h-full w-full items-center justify-center">
          <Spinner size={26} />
        </div>
      )}

      {error && (
        <PageErrorCard
          error={error}
          onRetry={() => void retry()}
        />
      )}

      {pageQuery.data && !error && (
        <ReaderImage
          key={`${pageQuery.data.url}:${pageQuery.dataUpdatedAt}`}
          onRetry={() => void retry()}
          src={pageQuery.data.url}
          alt={t("detail.pageAlt", { n: index + 1 })}
          draggable={false}
          onDoubleClick={() => {
            setZoom(1);
            setPan({ x: 0, y: 0 });
          }}
          style={{
            transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          }}
          className={cn(
            "max-h-full max-w-full touch-none select-none object-contain transition-transform duration-150",
            zoom > 1 ? "cursor-grab active:cursor-grabbing" : "cursor-default",
          )}
        />
      )}

      {zoom !== 1 && (
        <div className="absolute bottom-4 left-1/2 z-20 flex -translate-x-1/2 items-center gap-1.5 rounded-full border border-border bg-elevated/90 px-2.5 py-1 text-caption text-muted-foreground">
          <ZoomIn size={12} strokeWidth={1.75} />
          {Math.round(zoom * 100)}%
          <button
            onClick={() => applyZoom(1)}
            className="ml-1 flex items-center gap-1 rounded px-1 font-medium text-accent hover:underline"
          >
            <RefreshCw size={11} strokeWidth={1.75} /> {t("reader.resetZoom")}
          </button>
        </div>
      )}
    </div>
  );
}
