import { useState } from "react";
import { Link } from "react-router-dom";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Clock, Trash2 } from "lucide-react";
import { clearHistory, getHistory, removeHistory } from "../lib/api";
import { CachedImage } from "../components/ui/cached-image";
import { EmptyState } from "../components/ui/empty-state";
import { ErrorState } from "../components/ui/error-state";
import { ConfirmDialog } from "../components/ui/dialog";
import { Button } from "../components/ui/button";
import { useToastedMutation } from "../lib/use-toasted-mutation";
import { useT } from "../lib/i18n";

function formatTime(ts: number): string {
  const d = new Date(ts);
  const today = new Date();
  const sameDay = d.toDateString() === today.toDateString();
  return sameDay
    ? d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

const HISTORY_PAGE_SIZE = 50;

export function HistoryPage() {
  const t = useT();
  const [confirmClear, setConfirmClear] = useState(false);

  const query = useInfiniteQuery({
    queryKey: ["history"],
    queryFn: ({ pageParam }) => getHistory(pageParam),
    initialPageParam: 0,
    getNextPageParam: (last, pages) =>
      last.length >= HISTORY_PAGE_SIZE ? pages.length : undefined,
  });

  const entries = query.data?.pages.flat() ?? [];

  const removeMutation = useToastedMutation({
    mutationFn: (gid: number) => removeHistory(gid),
    successMessage: t("history.removed"),
    errorMessage: t("history.removeFailed"),
    onSuccess: () => void query.refetch(),
  });
  const clearMutation = useToastedMutation({
    mutationFn: () => clearHistory(),
    successMessage: t("history.cleared"),
    errorMessage: t("history.clearFailed"),
    onSuccess: () => void query.refetch(),
  });

  if (query.isLoading) {
    return (
      <div className="mx-auto w-full max-w-4xl p-4 pb-20 md:pb-4">
        <div className="mb-3 flex items-center justify-between">
          <div className="h-3.5 w-20 animate-pulse rounded bg-surface-2" />
          <div className="h-7 w-20 animate-pulse rounded-md bg-surface-2" />
        </div>
        <ul className="divide-y divide-border rounded-lg border border-border bg-surface">
          {Array.from({ length: 8 }, (_, i) => (
            <li key={i} className="flex items-center gap-3 p-2 pr-3">
              <div className="h-14 w-11 shrink-0 animate-pulse rounded-md bg-surface-2" />
              <div className="min-w-0 flex-1 space-y-1.5">
                <div className="h-4 w-2/3 animate-pulse rounded bg-surface-2" />
                <div className="h-3 w-24 animate-pulse rounded bg-surface-2" />
              </div>
            </li>
          ))}
        </ul>
      </div>
    );
  }

  if (query.isError) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <ErrorState
          description={t("history.loadError")}
          onRetry={() => void query.refetch()}
          className="w-full max-w-md"
        />
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <EmptyState
          icon={Clock}
          title={t("history.empty")}
          description={t("history.emptyDesc")}
          action={
            <Link
              to="/"
              className="inline-flex h-8 items-center rounded-md border border-border px-3 text-small text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground"
            >
              {t("history.start")}
            </Link>
          }
          className="w-full max-w-md"
        />
      </div>
    );
  }

  return (
    <div className="mx-auto h-full max-w-4xl overflow-y-auto p-4">
      <div className="mb-3 flex items-center justify-between">
        <p className="font-mono text-caption text-faint-foreground tabular-nums">
          {entries.length === 1
            ? t("history.entryCount", { n: entries.length })
            : t("history.entriesCount", { n: entries.length })}
        </p>
        <Button variant="outline" size="sm" onClick={() => setConfirmClear(true)}>
          {t("history.clearAll")}
        </Button>
      </div>
      <ul className="divide-y divide-border rounded-lg border border-border bg-surface">
        {entries.map((h) => (
          <li
            key={h.gid}
            data-density
            className="group flex items-center gap-3 p-[var(--density-row-pad)] pr-3"
          >
            <Link
              to={`/gallery/${h.gid}/${h.token}`}
              className="flex min-w-0 flex-1 items-center gap-3"
            >
              <CachedImage
                url={h.thumb}
                alt=""
                className="h-14 w-11 shrink-0 rounded-md object-cover"
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-base font-medium group-hover:text-accent">
                  {h.title}
                </span>
                <span className="block text-caption text-faint-foreground">{formatTime(h.time)}</span>
              </span>
            </Link>
            <button
              aria-label={t("history.remove", { title: h.title })}
              onClick={() => removeMutation.mutate(h.gid)}
              className="rounded-md p-1.5 text-faint-foreground opacity-0 transition-all duration-150 hover:bg-danger/10 hover:text-danger focus:opacity-100 group-hover:opacity-100"
            >
              <Trash2 size={14} strokeWidth={1.75} />
            </button>
          </li>
        ))}
      </ul>
      {query.hasNextPage && (
        <div className="flex justify-center pt-3">
          <Button
            variant="outline"
            size="sm"
            onClick={() => void query.fetchNextPage()}
            disabled={query.isFetchingNextPage}
          >
            {query.isFetchingNextPage ? t("common.loading") : t("history.loadMore")}
          </Button>
        </div>
      )}
      <ConfirmDialog
        open={confirmClear}
        onOpenChange={setConfirmClear}
        title={t("history.confirmTitle")}
        description={t("history.confirmDesc")}
        confirmLabel={t("history.clearAll")}
        onConfirm={() => clearMutation.mutate()}
      />
    </div>
  );
}
