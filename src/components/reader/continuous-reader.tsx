import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useT } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { useElementWidth } from "../../lib/use-element-width";
import { cn, isEditableTarget } from "../../lib/utils";
import { steppedZoom, useReaderZoom, type ZoomAction } from "../../lib/use-zoom-shortcuts";
import { Spinner } from "../ui/spinner";

import { PageErrorCard, ReaderImage } from "./page-image";
import { usePageQuery } from "./page-query";

export function ContinuousReader({
  controlsVisible,
  toolbarH,
  gid,
  token,
  index,
  pageCount,
  onIndexChange,
  onReachedEnd,
  fit,
}: {
  controlsVisible: boolean;
  toolbarH: number;
  gid: number;
  token: string;
  index: number;
  pageCount: number;
  onIndexChange: (i: number) => void;
  onReachedEnd: () => void;
  fit: "page" | "width";
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const width = useElementWidth(scrollRef);
  const preload = useSettings((s) => s.settings.readerPreloadCount);
  /**
   * A fling passes dozens of pages, and every one the virtualizer mounts would
   * claim a slot in the 500ms-spaced request queue — so the page the reader
   * actually stops on waits behind pages nobody will look at. Only pages near
   * a settled position are fetched.
   */
  const [settled, setSettled] = useState(index);
  useEffect(() => {
    const id = window.setTimeout(() => setSettled(index), 150);
    return () => window.clearTimeout(id);
  }, [index]);
  const [zoom, setZoom] = useState(1);
  useReaderZoom(useCallback((action: ZoomAction) => setZoom(value => steppedZoom(value, action, 0.5, 3)), []));
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const pinchDist = useRef<number | null>(null);

  /**
   * Vertical keys scroll the strip. Without this they reached the page-level
   * handler, which moves the page index — so Space jumped a whole page in the
   * one mode where the point is to read across the seam.
   */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (document.querySelector("dialog[open]") || e.ctrlKey || e.metaKey || e.altKey) return;
      if (isEditableTarget(e.target)) return;
      if (e.key === " " && e.target instanceof Element && e.target.closest("button, a")) return;
      const el = scrollRef.current;
      if (!el) return;
      const screen = el.clientHeight * 0.9;
      const delta =
        e.key === "ArrowDown" ? 120
        : e.key === "ArrowUp" ? -120
        : e.key === "PageDown" || e.key === " " ? (e.shiftKey ? -screen : screen)
        : e.key === "PageUp" ? -screen
        : null;
      if (delta === null) return;
      e.preventDefault();
      el.scrollTop += delta;
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const onPointerDown = (e: React.PointerEvent) => {
    pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.current.size === 2) {
      const [a, b] = [...pointers.current.values()];
      pinchDist.current = Math.hypot(a.x - b.x, a.y - b.y);
    }
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (!pointers.current.has(e.pointerId)) return;
    pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.current.size !== 2 || pinchDist.current == null) return;
    const [a, b] = [...pointers.current.values()];
    const dist = Math.hypot(a.x - b.x, a.y - b.y);
    if (pinchDist.current > 0) {
      setZoom((z) => Math.max(0.5, Math.min(3, Math.round(z * (dist / pinchDist.current!) * 100) / 100)));
    }
    pinchDist.current = dist;
  };

  const onPointerUp = (e: React.PointerEvent) => {
    pointers.current.delete(e.pointerId);
    if (pointers.current.size < 2) pinchDist.current = null;
  };

  const pageW = Math.max(0, fit === "width" ? width : Math.min(width - 24, 900)) * zoom;
  const pageH = pageW * 1.41 + 24;
  const estimate = useMemo(() => () => pageH, [pageH]);

  const virtualizer = useVirtualizer({
    count: pageCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: estimate,
    overscan: 3,
    scrollMargin: 0,
  });

  useLayoutEffect(() => { virtualizer.measure(); }, [pageH, virtualizer]);

  // The fixed-size canvas commits with pageH, so restoration never depends on
  // when the virtualizer refreshes its cached estimates or the next frame runs.
  const lastIndexRef = useRef(index);
  const restored = useRef(false);
  const lastWidthRef = useRef(0);
  const lastPageHRef = useRef(0);
  useLayoutEffect(() => {
    if (width <= 0) return;
    if (restored.current && index === lastIndexRef.current && width === lastWidthRef.current && pageH === lastPageHRef.current) return;
    lastPageHRef.current = pageH;
    lastIndexRef.current = index;
    lastWidthRef.current = width;
    if (scrollRef.current) scrollRef.current.scrollTop = index * pageH;
    restored.current = true;
  }, [index, width, pageH]);

  const currentFromScroll = useCallback(() => {
    if (!restored.current || pageH <= 0) return;
    const el = scrollRef.current;
    if (!el) return;
    const inset = controlsVisible ? toolbarH : 0;
    const position = Math.max(0, el.scrollTop + el.clientHeight / 2 - inset);
    const current = Math.min(pageCount - 1, Math.floor(position / pageH));
    if (current !== lastIndexRef.current) {
      lastIndexRef.current = current;
      onIndexChange(current);
    }
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 8) onReachedEnd();
  }, [pageH, pageCount, onIndexChange, onReachedEnd, controlsVisible, toolbarH]);

  // Toggling the chrome changes the top padding, which would otherwise slide
  // the page the reader is looking at by the height of the bar.
  const padRef = useRef(controlsVisible ? toolbarH : 0);
  useLayoutEffect(() => {
    const pad = controlsVisible ? toolbarH : 0;
    const delta = pad - padRef.current;
    padRef.current = pad;
    if (delta !== 0 && scrollRef.current) scrollRef.current.scrollTop += delta;
  }, [controlsVisible, toolbarH]);

  return (
    <div
      ref={scrollRef}
      data-reader-zoom={zoom}
      onScroll={currentFromScroll}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      style={{ paddingTop: controlsVisible ? toolbarH : 0, touchAction: "pan-y" }}
      className={cn("min-h-0 flex-1 overflow-auto")}
    >
      <div
        className="relative mx-auto flex flex-col items-center"
        style={{ height: pageCount * pageH, width: pageW }}
      >
        {virtualizer.getVirtualItems().map((row) => (
          <div
            key={row.key}
            className="absolute left-0 top-0 w-full"
            style={{ transform: `translateY(${row.start}px)` }}
          >
            <ContinuousPage
              gid={gid}
              token={token}
              index={row.index}
              width={pageW}
              enabled={Math.abs(row.index - settled) <= Math.max(1, preload)}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

function ContinuousPage({
  gid,
  token,
  index,
  width,
  enabled,
}: {
  gid: number;
  token: string;
  index: number;
  enabled: boolean;
  width: number;
}) {
  const t = useT();
  const { query: pageQuery, retry } = usePageQuery(gid, token, index, enabled);
  return (
    <div
      className="mb-6 flex items-center justify-center bg-surface"
      style={{ width, height: width * 1.41 }}
    >
      {pageQuery.isPending && <Spinner size={22} />}
      {pageQuery.isError && (
        <PageErrorCard error={pageQuery.error} onRetry={() => void retry()} />
      )}
      {pageQuery.data && (
        <ReaderImage
          key={`${pageQuery.data.url}:${pageQuery.dataUpdatedAt}`}
          onRetry={() => void retry()}
          src={pageQuery.data.url}
          alt={t("detail.pageAlt", { n: index + 1 })}
          loading="lazy"
          className="h-full w-full object-contain"
        />
      )}
    </div>
  );
}
