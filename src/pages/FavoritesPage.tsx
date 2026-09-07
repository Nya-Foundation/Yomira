import { nextListPage } from "../lib/pagination";
import type { ListPage } from "../lib/types";
import { useState } from "react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { getFavorites, checkSignIn, isNotSignedIn, type FavoriteSort } from "../lib/api";
import { cn } from "../lib/utils";
import { GalleryGrid } from "../components/gallery/gallery-grid";
import { EmptyState } from "../components/ui/empty-state";
import { FilterChip } from "../components/ui/filter-chip";
import { GhostLink } from "../components/ui/button";
import { Inbox } from "lucide-react";
import type { FavoriteItem } from "../lib/types";
import { useT } from "../lib/i18n";

export function FavoritesPage() {
  const t = useT();
  const [slot, setSlot] = useState<number | null>(null);
  const [sort, setSort] = useState<FavoriteSort>("f");

  return (
    <div className="flex h-full flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border px-4 py-3">
        <div className="flex items-center rounded-md border border-border p-0.5">
          {(["f", "p"] as const).map((s) => (
            <button
              key={s}
              onClick={() => setSort(s)}
              aria-pressed={sort === s}
              className={cn(
                "h-6 rounded px-2.5 text-small transition-colors duration-150",
                sort === s
                  ? "bg-accent/15 text-accent"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {s === "f" ? t("favorites.sort.favorited") : t("favorites.sort.posted")}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-1.5 overflow-x-auto">
          <FilterChip active={slot === null} onClick={() => setSlot(null)} label={t("favorites.all")} />
          {Array.from({ length: 10 }, (_, i) => (
            <FilterChip key={i} active={slot === i} onClick={() => setSlot(i)} label={`${i + 1}`} />
          ))}
        </div>
      </div>
      <OnlineFavorites slot={slot} sort={sort} />
    </div>
  );
}

function OnlineFavorites({ slot, sort }: { slot: number | null; sort: FavoriteSort }) {
  const t = useT();
  const { data: signedIn } = useQuery({ queryKey: ["signin"], queryFn: checkSignIn });

  const query = useInfiniteQuery({
    queryKey: ["favorites", slot, sort],
    queryFn: ({ pageParam }) => getFavorites(pageParam, slot, sort),
    initialPageParam: 0 as ListPage,
    getNextPageParam: (last, _pages, _param, params) => nextListPage(last.nextPage, params),
    retry: 1,
  });

  const items = (query.data?.pages.flatMap((p) => p.items) ?? []) as FavoriteItem[];
  const needSignIn = query.isError && isNotSignedIn(query.error);

  if (needSignIn) {
    return (
      <div className="flex flex-1 p-4">
        <EmptyState
          icon={Inbox}
          title={t("favorites.signInTitle")}
          description={t("favorites.signInDesc")}
          action={<GhostLink to="/signin">{t("common.signIn")}</GhostLink>}
        />
      </div>
    );
  }


  return (
    <GalleryGrid
      className="flex-1"
      galleries={items}
      loading={query.isLoading}
      error={query.isError}
      onRetry={() => void (query.isFetchNextPageError ? query.fetchNextPage() : query.refetch())}
      hasNextPage={query.hasNextPage ?? false}
      isFetchingNextPage={query.isFetchingNextPage}
      onLoadMore={() => void query.fetchNextPage()}
      emptyTitle={signedIn === false ? t("favorites.demoEmpty") : t("favorites.empty")}
      emptyDescription={
        signedIn === false
          ? t("favorites.demoEmptyDesc")
          : t("favorites.emptyDesc")
      }
      emptyAction={<GhostLink to="/">{t("favorites.browse")}</GhostLink>}
    />
  );
}
