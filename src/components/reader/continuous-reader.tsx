import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";
import { steppedZoom, useReaderZoom, type ZoomAction } from "../../lib/use-zoom-shortcuts";
import { Spinner } from "../ui/spinner";

import { PageErrorCard, ReaderImage } from "./page-image";
import { usePageQuery } from "./page-query";

export function ContinuousReader({
  controlsVisible,
  gid,
  token,
  index,
  pageCount,
  onIndexChange,
}: {
  controlsVisible: boolean;
  gid: number;
  token: string;
  index: number;
  pageCount: number;
  onIndexChange: (i: number) => void;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [zoom, setZoom] = useState(1);
  useReaderZoom(useCallback((action: ZoomAction) => setZoom(value => steppedZoom(value, action, 0.5, 3)), []));

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      setWidth(entries[0]?.contentRect.width ?? 0);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const pageW = Math.max(0, Math.min(width - 24, 900)) * zoom;
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
    const inset = controlsVisible ? 6 * parseFloat(getComputedStyle(el).fontSize) : 0;
    const position = Math.max(0, el.scrollTop + el.clientHeight / 2 - inset);
    const current = Math.min(pageCount - 1, Math.floor(position / pageH));
    if (current !== lastIndexRef.current) {
      lastIndexRef.current = current;
      onIndexChange(current);
    }
  }, [pageH, pageCount, onIndexChange, controlsVisible]);

  return (
    <div ref={scrollRef} data-reader-zoom={zoom} onScroll={currentFromScroll} className={cn("min-h-0 flex-1 overflow-auto", controlsVisible && "pt-24")}>
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
}: {
  gid: number;
  token: string;
  index: number;
  width: number;
}) {
  const t = useT();
  const { query: pageQuery, retry } = usePageQuery(gid, token, index, true);
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
