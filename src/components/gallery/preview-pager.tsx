import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { getPreviewSet } from "../../lib/api";
import type { PreviewSet } from "../../lib/types";
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
            <div key={i} className="aspect-[3/4] animate-pulse rounded-md bg-surface-2" />
          ))}
      </div>
      {groups > 1 && (
        <div className="mt-3 flex items-center justify-center gap-1">
          {Array.from({ length: Math.min(groups, 12) }, (_, i) => (
            <button
              key={i}
              onClick={() => setGroup(i)}
              aria-label={t("detail.previewGroup", { n: i + 1 })}
              aria-current={i === group}
              className={
                i === group
                  ? "h-6 min-w-6 rounded-md bg-accent/20 px-1 text-caption font-medium text-accent"
                  : "h-6 min-w-6 rounded-md px-1 text-caption text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground"
              }
            >
              {i + 1}
            </button>
          ))}
          {groups > 12 && (
            <span className="text-caption text-faint-foreground">
              {t("detail.moreGroups", { n: groups - 12 })}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
