import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { X } from "lucide-react";
import { clearSearchHistory, deleteSearchTerm, searchHistory } from "../../lib/api";
import { useT } from "../../lib/i18n";

/**
 * Dropdown of recent searches, newest first. Shown while the search input is
 * focused and empty; clicking a term applies it, the per-item X removes just
 * that term, and the header action clears everything.
 */
export function SearchHistoryDropdown({
  open,
  onApply,
}: {
  open: boolean;
  onApply: (term: string) => void;
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["search-history"], queryFn: searchHistory, enabled: open });
  const invalidate = () => void queryClient.invalidateQueries({ queryKey: ["search-history"] });
  const remove = useMutation({ mutationFn: deleteSearchTerm, onSuccess: invalidate });
  const clearAll = useMutation({ mutationFn: clearSearchHistory, onSuccess: invalidate });

  if (!open) return null;
  const items = query.data ?? [];
  if (items.length === 0) return null;

  return (
    <div className="absolute left-0 right-0 top-full z-50 mt-1.5 overflow-hidden rounded-lg border border-border bg-elevated shadow-xl shadow-black/40">
      <div className="flex items-center justify-between px-3 pt-2 pb-1">
        <span className="text-caption text-faint-foreground">{t("searchhistory.title")}</span>
        <button
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => clearAll.mutate()}
          className="text-caption text-faint-foreground transition-colors hover:text-danger"
        >
          {t("searchhistory.clearAll")}
        </button>
      </div>
      {items.map((term) => (
        <div key={term} className="group/hist flex items-center">
          <button
            onMouseDown={(e) => {
              e.preventDefault();
              onApply(term);
            }}
            aria-label={t("searchhistory.apply", { term })}
            className="min-w-0 flex-1 truncate px-3 py-1.5 text-left text-base text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground"
          >
            {term}
          </button>
          <button
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => remove.mutate(term)}
            aria-label={t("searchhistory.delete", { term })}
            className="shrink-0 rounded-full p-1 pr-2.5 text-faint-foreground opacity-0 transition-opacity hover:text-danger group-hover/hist:opacity-100"
          >
            <X size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}
