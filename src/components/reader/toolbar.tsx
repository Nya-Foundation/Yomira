import { useEffect, useRef, useState } from "react";
import {
  BookOpen,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Rows3
} from "lucide-react";
import { NavigationControls } from "../layout/navigation-controls";
import { useT } from "../../lib/i18n";
import { Button } from "../ui/button";
import { Slider } from "../ui/slider";
import { Tooltip } from "../ui/tooltip";

export function ReaderToolbar({
  hidden,
  setHidden,
  index,
  pageCount,
  mode,
  onModeChange,
  onIndexChange,
  title,
  offlineStatus,
  onHeight,
}: {
  hidden: boolean;
  setHidden: (hidden: boolean) => void;
  index: number;
  pageCount: number;
  mode: "paged" | "continuous";
  onModeChange: (m: "paged" | "continuous") => void;
  onIndexChange: (i: number) => void;
  title: string;
  offlineStatus?: string;
  /** Reports the bar's measured height so the reader can reserve exactly it. */
  onHeight?: (height: number) => void;
}) {
  const t = useT();
  // Every intermediate value of a drag would reserve its own throttled page
  // request, leaving the page actually released on at the back of the queue.
  // The scrubbed value is shown live and committed once, on release.
  const [scrub, setScrub] = useState<number | null>(null);
  const shown = scrub ?? index;
  useEffect(() => setScrub(null), [index]);
  const barRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = barRef.current;
    if (!el || !onHeight) return;
    const observer = new ResizeObserver(() => onHeight(el.getBoundingClientRect().height));
    observer.observe(el);
    onHeight(el.getBoundingClientRect().height);
    return () => observer.disconnect();
  }, [hidden, onHeight]);

  const commit = () => {
    if (scrub !== null && scrub !== index) onIndexChange(scrub);
    setScrub(null);
  };
  if (hidden) return (
    <button onClick={() => setHidden(false)} aria-label={t("reader.showControls")}
      className="absolute right-3 top-3 z-30 flex size-11 items-center justify-center rounded-lg border border-border bg-surface text-foreground">
      <ChevronDown size={18} />
    </button>
  );

  return (
    <div ref={barRef} className="absolute inset-x-0 top-0 z-30 border-b border-border bg-background">
      <div className="flex min-h-14 items-center gap-2 px-3">
        <NavigationControls />
        <div className="min-w-0 flex-1">
          <div className="truncate text-base text-muted-foreground">{title}</div>
          {offlineStatus && <div className="truncate text-caption text-muted-foreground" role="status">{offlineStatus}</div>}
        </div>
        <span role="status" aria-atomic="true" className="shrink-0 text-small text-muted-foreground tabular-nums">
          {t("reader.pageOf", { n: shown + 1, total: pageCount })}
        </span>
        <Tooltip content={mode === "paged" ? t("reader.toContinuous") : t("reader.toPaged")} side="bottom">
          <Button variant="ghost" size="icon" className="min-h-[44px] min-w-[44px]" onClick={() => onModeChange(mode === "paged" ? "continuous" : "paged")} aria-label={t("reader.toggleMode")}>
            {mode === "paged" ? <Rows3 size={18} /> : <BookOpen size={18} />}
          </Button>
        </Tooltip>
        <Button variant="ghost" size="icon" className="min-h-[44px] min-w-[44px]" onClick={() => setHidden(true)} aria-label={t("reader.hideControls")}><ChevronUp size={18} /></Button>
      </div>
      <div className="flex min-h-10 items-center gap-3 px-4 pb-2">
        <Button variant="ghost" size="icon" disabled={index === 0} onClick={() => onIndexChange(index - 1)} aria-label={t("reader.prevPage")}><ChevronLeft size={16} /></Button>
        <Slider className="min-w-0 flex-1" min={0} max={Math.max(pageCount - 1, 1)} value={shown}
          onChange={(e) => setScrub(Number(e.target.value))}
          onPointerUp={commit} onKeyUp={commit} onBlur={commit}
          aria-label={t("reader.page")} />
        <Button variant="ghost" size="icon" disabled={index === pageCount - 1} onClick={() => onIndexChange(index + 1)} aria-label={t("reader.nextPage")}><ChevronRight size={16} /></Button>
      </div>
    </div>
  );
}
