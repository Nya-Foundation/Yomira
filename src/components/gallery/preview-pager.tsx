import { useEffect, useState } from "react";
import { Skeleton } from "../ui/skeleton";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { getPreviewSet } from "../../lib/api";
import type { PreviewSet } from "../../lib/types";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useT } from "../../lib/i18n";
import { CachedImage } from "../ui/cached-image";

/** Preview thumbnail grid with pager buttons for every group of 20 pages. */
export function PreviewPager({
  gid,
  token,
  total,
  initial,
}: {
  gid: number;
  token: string;
  total: number;
  initial: PreviewSet;
}) {
  const t = useT();
  const [group, setGroup] = useState(0);
  const groups = Math.ceil(total / 20);
  const query = useQuery({
    queryKey: ["preview-set", gid, group],
    queryFn: () => getPreviewSet(gid, token, group),
    placeholderData: group === 0 ? () => initial : undefined,
  });

  useEffect(() => {
    setGroup(0);
  }, [gid]);

  return (
    <div>
      <div className="grid grid-cols-4 gap-2 sm:grid-cols-6 md:grid-cols-8 lg:grid-cols-10">
        {query.data?.previews.map((p) => (
          <Link
            key={p.index}
            to={`/read/${gid}/${token}?p=${p.index}`}
            className="block overflow-hidden rounded-md border border-border bg-surface-2 transition-[border-color] duration-150 hover:border-border-strong"
          >
            <CachedImage
              url={p.thumb}
              alt={t("detail.pageAlt", { n: p.index + 1 })}
              style={{ aspectRatio: `${p.width}/${p.height}` }}
              className="w-full object-cover"
            />
          </Link>
        ))}
        {query.isLoading &&
          Array.from({ length: 20 }, (_, i) => (
            <Skeleton key={i} className="aspect-[3/4]" />
          ))}
      </div>
      {groups > 1 && (
        <div className="mt-3 flex items-center justify-center gap-1">
          <button
            onClick={() => setGroup(Math.max(0, group - 1))}
            disabled={group === 0}
            aria-label={t("detail.previewPrevGroup")}
            className="h-6 rounded-md px-1 text-caption text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground disabled:opacity-40"
          >
            <ChevronLeft size={14} />
          </button>
          {groupWindow(group, groups).map((entry, i) =>
            entry === "gap" ? (
              <span key={`gap${i}`} aria-hidden className="px-0.5 text-caption text-faint-foreground">
                &hellip;
              </span>
            ) : (
              <button
                key={entry}
                onClick={() => setGroup(entry)}
                aria-label={t("detail.previewGroup", { n: entry + 1 })}
                aria-current={entry === group}
                className={
                  entry === group
                    ? "h-6 min-w-6 rounded-md bg-accent/20 px-1 text-caption font-medium text-accent"
                    : "h-6 min-w-6 rounded-md px-1 text-caption text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground"
                }
              >
                {entry + 1}
              </button>
            ),
          )}
          <button
            onClick={() => setGroup(Math.min(groups - 1, group + 1))}
            disabled={group === groups - 1}
            aria-label={t("detail.previewNextGroup")}
            className="h-6 rounded-md px-1 text-caption text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground disabled:opacity-40"
          >
            <ChevronRight size={14} />
          </button>
        </div>
      )}
    </div>
  );
}

/**
 * Group numbers to show: always the first and last, plus a span either side of
 * the current one, with a gap marker where numbers are skipped. The pager used
 * to render the first twelve and then state how many more existed without
 * offering any way to reach them, so a 383-page gallery had no preview access
 * past page 240.
 */
function groupWindow(current: number, total: number, span = 2): (number | "gap")[] {
  const want = new Set<number>([0, total - 1]);
  for (let i = current - span; i <= current + span; i++) {
    if (i >= 0 && i < total) want.add(i);
  }
  const out: (number | "gap")[] = [];
  let previous = -1;
  for (const n of [...want].sort((a, b) => a - b)) {
    if (previous >= 0 && n - previous > 1) out.push("gap");
    out.push(n);
    previous = n;
  }
  return out;
}
