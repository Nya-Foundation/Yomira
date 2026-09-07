import { useInfiniteQuery } from "@tanstack/react-query";
import { ExternalLink, Newspaper } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getNews } from "../lib/api";
import { formatRelativeTime } from "../lib/utils";
import { useT } from "../lib/i18n";
import { Button } from "../components/ui/button";
import { EmptyState } from "../components/ui/empty-state";
import { ErrorState } from "../components/ui/error-state";
import { Spinner } from "../components/ui/spinner";

/** Backend page size for get_news; a short page means no more items. */
const NEWS_PAGE_SIZE = 20;

async function openExternal(link: string) {
  try {
    await openUrl(link);
  } catch {
    window.open(link, "_blank", "noopener");
  }
}

/** `date` arrives as an ISO-ish string; fall back to the raw value. */
function formatNewsDate(date: string): string {
  const ts = Date.parse(date);
  return Number.isNaN(ts) ? date : formatRelativeTime(ts / 1000);
}

export function NewsPage() {
  const t = useT();
  const query = useInfiniteQuery({
    queryKey: ["news"],
    queryFn: ({ pageParam }) => getNews(pageParam),
    initialPageParam: 0,
    getNextPageParam: (last, pages) =>
      last.length >= NEWS_PAGE_SIZE ? pages.length : null,
  });

  const items = query.data?.pages.flatMap((p) => p) ?? [];

  if (query.isLoading) {
    return (
      <div className="mx-auto max-w-3xl space-y-3 p-6">
        {Array.from({ length: 5 }, (_, i) => (
          <div key={i} className="space-y-2 rounded-lg border border-border bg-surface p-4">
            <div className="h-4 w-1/3 animate-pulse rounded bg-surface-2" />
            <div className="h-3 w-24 animate-pulse rounded bg-surface-2" />
            <div className="h-3 w-full animate-pulse rounded bg-surface-2" />
            <div className="h-3 w-2/3 animate-pulse rounded bg-surface-2" />
          </div>
        ))}
      </div>
    );
  }

  if (query.isError) {
    return (
      <ErrorState
        description={t("news.errorDesc")}
        onRetry={() => void query.refetch()}
      />
    );
  }

  if (items.length === 0) {
    return (
      <EmptyState
        icon={Newspaper}
        title={t("news.empty")}
        description={t("news.emptyDesc")}
      />
    );
  }

  return (
    <div className="mx-auto h-full max-w-3xl overflow-y-auto p-4 sm:p-6">
      <ul className="space-y-3">
        {items.map((item, i) => (
          <li key={`${item.title}-${i}`} className="rounded-lg border border-border bg-surface p-4">
            <div className="flex items-start justify-between gap-3">
              <h2 className="text-emphasis font-semibold leading-6 tracking-tight">{item.title}</h2>
              {item.link && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void openExternal(item.link)}
                  aria-label={t("news.openLink")}
                  className="shrink-0 gap-1"
                >
                  <ExternalLink size={13} strokeWidth={1.75} />
                  {t("news.openLink")}
                </Button>
              )}
            </div>
            <p className="mt-0.5 text-caption text-faint-foreground">{formatNewsDate(item.date)}</p>
            <p className="mt-2 text-small leading-6 text-muted-foreground">{item.body}</p>
          </li>
        ))}
      </ul>
      <div className="flex h-14 items-center justify-center">
        {query.isFetchingNextPage ? (
          <Spinner size={18} />
        ) : (
          query.hasNextPage && (
            <Button variant="outline" size="sm" onClick={() => void query.fetchNextPage()}>
              {t("common.loadMore")}
            </Button>
          )
        )}
      </div>
    </div>
  );
}
