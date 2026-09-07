import { nextListPage } from "../lib/pagination";
import type { ListPage } from "../lib/types";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { ArrowRightToLine, Filter, Image as ImageIcon, Info, Plus, Search, SlidersHorizontal, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { GalleryGrid } from "../components/gallery/gallery-grid";
import { QuotaBar } from "../components/gallery/quota-bar";
import { AdvancedSearchSheet, CategorySheet } from "../components/search/filter-sheets";
import { ImageSearchDialog, type ImageSearchResult } from "../components/search/image-search-dialog";
import { SearchHistoryDropdown } from "../components/search/search-history";
import { useTagAutocomplete } from "../components/search/tag-autocomplete";
import { Button } from "../components/ui/button";
import { Dialog } from "../components/ui/dialog";
import { Input } from "../components/ui/input";
import { Tooltip } from "../components/ui/tooltip";
import {
  checkSignIn,
  createQuickSearch,
  deleteQuickSearch,
  getGalleryList,
  quickSearchList,
  recordSearch,
  type ListMode,
} from "../lib/api";
import { useT } from "../lib/i18n";
import {
  DEFAULT_LIST_PARAMS,
  type GalleryInfo,
  type ListParams
} from "../lib/types";
import { useDebounced } from "../lib/use-debounced";
import { useToastedMutation } from "../lib/use-toasted-mutation";
import { cn } from "../lib/utils";

interface SavedListState {
  params: ListParams;
  searchInput: string;
  startPage: number;
}

/** Per-mode list state (search text, filters, jump target) so navigating
 * into a gallery/reader and back restores the exact previous state. */
function loadListState(mode: string, entry: string): SavedListState | null {
  try {
    const raw = sessionStorage.getItem(`exviewer.liststate.${mode}.${entry}`) ?? sessionStorage.getItem(`exviewer.liststate.${mode}`);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<SavedListState>;
    if (typeof parsed.searchInput !== "string" || typeof parsed.params !== "object" || parsed.params === null) {
      return null;
    }
    return {
      params: { ...DEFAULT_LIST_PARAMS, ...parsed.params },
      searchInput: parsed.searchInput,
      startPage: Number(parsed.startPage) || 0,
    };
  } catch {
    return null;
  }
}

function saveListState(mode: string, entry: string, state: SavedListState) {
  try {
    sessionStorage.setItem(`exviewer.liststate.${mode}`, JSON.stringify(state));
    sessionStorage.setItem(`exviewer.liststate.${mode}.${entry}`, JSON.stringify(state));
  } catch {
    /* storage full or unavailable — state restore is best-effort */
  }
}

export function GalleryListPage({ mode }: { mode: ListMode }) {
  const t = useT();
  const location = useLocation();
  const [saved] = useState(() => loadListState(mode, location.key));
  const [params, setParams] = useState<ListParams>(() => ({ ...(saved?.params ?? DEFAULT_LIST_PARAMS), search: new URLSearchParams(location.search).get("search") ?? saved?.searchInput ?? "" }));
  const [searchInput, setSearchInput] = useState<string>(
    () => new URLSearchParams(window.location.search).get("search") ?? saved?.searchInput ?? "",
  );
  const search = useDebounced(searchInput, 300);
  const [catSheetOpen, setCatSheetOpen] = useState(false);
  const [advSheetOpen, setAdvSheetOpen] = useState(false);
  const [newQuick, setNewQuick] = useState("");
  const [imgSearchOpen, setImgSearchOpen] = useState(false);
  const [imageResults, setImageResults] = useState<ImageSearchResult | null>(null);
  const [histFocused, setHistFocused] = useState(false);
  const [startPage, setStartPage] = useState(saved?.startPage ?? 0);
  const [jumpOpen, setJumpOpen] = useState(false);
  const [jumpValue, setJumpValue] = useState("1");
  const searchRef = useRef<HTMLInputElement>(null);
  const autocomplete = useTagAutocomplete({
    query: searchInput,
    inputRef: searchRef,
    onApply: (nextText) => setSearchInput(nextText),
  });

  useEffect(() => {
    const linkedSearch = new URLSearchParams(location.search).get("search");
    if (linkedSearch !== null) setSearchInput(linkedSearch);
    if (location.state?.focusSearch) searchRef.current?.focus();
  }, [location.search, location.key, location.state]);

  const quickQuery = useQuery({ queryKey: ["quick-searches"], queryFn: quickSearchList });
  const quickEntries = quickQuery.data ?? [];

  const addQuick = useToastedMutation({
    mutationFn: (keyword: string) => createQuickSearch({ name: keyword, keyword }),
    successMessage: (_data, keyword) => t("list.quickAdded", { name: keyword }),
    errorMessage: t("list.quickAddFailed"),
    invalidateKeys: [["quick-searches"]],
  });
  const removeQuick = useToastedMutation({
    mutationFn: ({ id }: { id: number; name: string }) => deleteQuickSearch(id),
    successMessage: (_data, vars) => t("list.quickDeleted", { name: vars.name }),
    errorMessage: t("list.quickDeleteFailed"),
    invalidateKeys: [["quick-searches"]],
  });

  useEffect(() => {
    setParams((p) => ({ ...p, search }));
    // The debounced value landing here is the point where a search actually
    // executes; that is when the term enters the history.
    if (search.trim()) recordSearch(search.trim());
  }, [search]);

  // any change to the search params or mode starts the listing back at page 1.
  // Identity changes don't count (the debounced search re-sets params with the
  // same values on mount) — only real content changes reset the jump target.
  const paramsContent = JSON.stringify(params);
  const prevParamsContent = useRef<string | null>(null);
  useEffect(() => {
    if (prevParamsContent.current === null) {
      prevParamsContent.current = paramsContent;
      return;
    }
    if (prevParamsContent.current !== paramsContent) {
      prevParamsContent.current = paramsContent;
      setStartPage(0);
    }
  }, [paramsContent, mode]);

  // persist the browsing state so back-navigation restores it verbatim
  useEffect(() => {
    saveListState(mode, location.key, { params, searchInput, startPage });
  }, [mode, location.key, params, searchInput, startPage]);

  useEffect(() => {
    const onFocus = () => searchRef.current?.focus();
    window.addEventListener("yomira:focus-search", onFocus);
    return () => window.removeEventListener("yomira:focus-search", onFocus);
  }, []);

  const query = useInfiniteQuery({
    queryKey: ["gallery-list", mode, params, startPage],
    queryFn: ({ pageParam }) => getGalleryList({ ...params, page: pageParam }, mode),
    initialPageParam: startPage as ListPage,
    getNextPageParam: (last, _pages, _param, params) => nextListPage(last.nextPage, params),
    retry: 1,
  });

  // e-hentai caps search browsing around page 50; fall back to an estimate
  // from the result total when the backend does not report a page count.
  const firstPage = query.data?.pages[0];
  const estimatedPages = firstPage ? Math.max(1, Math.ceil(firstPage.total / 25)) : 1;
  const maxJumpPage = Math.max(1, Math.min(firstPage?.pages || estimatedPages, 50));

  const applyJump = () => {
    const n = Math.round(Number(jumpValue));
    if (Number.isNaN(n)) return;
    setStartPage(Math.max(1, Math.min(n, maxJumpPage)) - 1);
    setJumpOpen(false);
  };

  // New uploads shift e-hentai result ordering between page fetches, so
  // consecutive pages overlap; keep the first occurrence per gid.
  const galleries = useMemo(() => {
    const seen = new Set<number>();
    const out: GalleryInfo[] = [];
    for (const page of query.data?.pages ?? []) {
      for (const g of page.galleries) {
        if (seen.has(g.gid)) continue;
        seen.add(g.gid);
        out.push(g);
      }
    }
    return out;
  }, [query.data]);
  // distinct searches restore their own scroll offset on back-navigation
  const scrollKey = `${mode}|${location.pathname}${location.search}|${JSON.stringify(params)}|${startPage}`;
  const activeCatCount = params.categories?.length ?? 0;
  const advActive =
    params.minRating != null ||
    params.minPages != null ||
    params.maxPages != null ||
    !params.enableUploader ||
    !params.enableTags ||
    params.searchDesc ||
    params.searchTorrent ||
    params.searchLowPowerTags ||
    params.searchDownvotedTags ||
    params.searchExpunged;

  return (
    <div className="flex h-full flex-col">
      {mode === "home" && location.pathname === "/" && <SignInBanner />}
      {mode === "home" && location.pathname === "/" && <QuotaBar />}

      {/* toolbar */}
      <div className="flex shrink-0 flex-col gap-2 border-b border-border px-4 pb-3 pt-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative w-full min-w-0 sm:w-auto sm:flex-1">
            <Search
              size={14}
              className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint-foreground"
            />
            <Input
              {...autocomplete.inputProps}
              ref={searchRef}
              value={searchInput}
              onChange={(e) => { setSearchInput(e.target.value); autocomplete.onSelect(e); }}
              onSelect={autocomplete.onSelect}
              onKeyDown={autocomplete.onKeyDown}
              onFocus={() => {
                autocomplete.onFocus();
                setHistFocused(true);
              }}
              onBlur={() => {
                autocomplete.onBlur();
                setHistFocused(false);
              }}
              placeholder={t("list.searchPlaceholder")}
              className="pl-8 pr-9"
              aria-label={t("list.searchAria")}
            />
            {autocomplete.dropdown}
            <SearchHistoryDropdown open={histFocused && !searchInput} onApply={setSearchInput} />
            {searchInput && (
              <button
                onClick={() => setSearchInput("")}
                aria-label={t("list.clearSearch")}
                className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-faint-foreground hover:text-foreground"
              >
                <X size={14} />
              </button>
            )}
          </div>
          <Button
            variant={activeCatCount ? "default" : "outline"}
            size="md"
            onClick={() => setCatSheetOpen(true)}
            aria-label={t("list.categoryFilters")}
          >
            <Filter size={14} />
            {t("list.categories")}
            {activeCatCount > 0 && (
              <span className="rounded bg-accent/25 px-1 text-caption">{activeCatCount}</span>
            )}
          </Button>
          <Button
            variant="outline"
            size="md"
            onClick={() => setImgSearchOpen(true)}
            aria-label={t("imagesearch.aria")}
          >
            <ImageIcon size={14} />
          </Button>
          {!(firstPage?.pages === 0 && typeof firstPage.nextPage === "string") && <Tooltip content={t("jump.title")} side="bottom">
            <span tabIndex={0} className="inline-flex">
              <Button
                variant="outline"
                size="md"
                onClick={() => setJumpOpen(true)}
                aria-label={t("jump.title")}
              >
                <ArrowRightToLine size={14} />
              </Button>
            </span>
          </Tooltip>}
          <Button
            variant={advActive ? "default" : "outline"}
            size="md"
            onClick={() => setAdvSheetOpen(true)}
            aria-label={t("list.advanced")}
          >
            <SlidersHorizontal size={14} />
            <span className="hidden sm:inline">{t("list.advanced")}</span>
          </Button>
        </div>

        {/* quick search chips */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-0.5">
          <span className="shrink-0 text-small text-faint-foreground">{t("list.quick")}</span>
          {quickEntries.map((q) => (
            <span
              key={q.id}
              className={cn(
                "group/chip flex shrink-0 items-center rounded-full border text-small transition-colors duration-150",
                searchInput.includes(q.keyword)
                  ? "border-accent/40 bg-accent/15 text-accent"
                  : "border-border bg-surface text-muted-foreground hover:border-border-strong hover:text-foreground",
              )}
            >
              <button
                onClick={() =>
                  setSearchInput((cur) => (cur.includes(q.keyword) ? cur : `${cur} ${q.keyword}`.trim()))
                }
                className="px-2 py-0.5"
                aria-label={t("list.applyQuickSearch", { name: q.name })}
              >
                {q.name}
              </button>
              <button
                onClick={() => removeQuick.mutate({ id: q.id, name: q.name })}
                aria-label={t("list.deleteQuickSearch", { name: q.name })}
                className="rounded-full p-0.5 pr-1.5 text-faint-foreground opacity-0 transition-opacity hover:text-danger focus:opacity-100 group-hover/chip:opacity-100"
              >
                <X size={11} />
              </button>
            </span>
          ))}
          <form
            className="flex shrink-0 items-center gap-1"
            onSubmit={(e) => {
              e.preventDefault();
              const v = newQuick.trim();
              if (v && !quickEntries.some((q) => q.keyword === v)) addQuick.mutate(v);
              setNewQuick("");
            }}
          >
            <input
              value={newQuick}
              onChange={(e) => setNewQuick(e.target.value)}
              placeholder={t("list.addQuickPlaceholder")}
              aria-label={t("list.addQuickFilter")}
              className="h-6 w-24 rounded-full border border-dashed border-border bg-transparent px-2 text-small text-muted-foreground placeholder:text-faint-foreground focus:border-accent focus:outline-none"
            />
            <button
              type="submit"
              aria-label={t("list.addQuickAria")}
              className="rounded-full p-0.5 text-faint-foreground hover:text-foreground"
            >
              <Plus size={13} />
            </button>
          </form>
        </div>
      </div>

      {imageResults ? (
        <>
          <div className="flex shrink-0 items-center gap-2 border-b border-accent/20 bg-accent/5 px-4 py-1.5">
            <ImageIcon size={13} strokeWidth={1.75} className="shrink-0 text-accent" />
            <span className="min-w-0 flex-1 truncate text-small text-muted-foreground">
              {t("imagesearch.resultsFor", { name: imageResults.filename })}
            </span>
            <button
              onClick={() => setImageResults(null)}
              className="flex shrink-0 items-center gap-1 rounded px-1.5 py-0.5 text-caption text-muted-foreground transition-colors hover:text-foreground"
            >
              <X size={12} />
              {t("imagesearch.clear")}
            </button>
          </div>
          <GalleryGrid
            className="flex-1"
            galleries={imageResults.response.galleries}
            loading={false}
            error={false}
            onRetry={() => {}}
            hasNextPage={false}
            isFetchingNextPage={false}
            onLoadMore={() => {}}
            emptyTitle={t("imagesearch.empty")}
            emptyDescription={t("imagesearch.emptyDesc")}
          />
        </>
      ) : (
        <GalleryGrid
          className="flex-1"
          scrollKey={scrollKey}
          galleries={galleries}
          loading={query.isLoading}
          error={query.isError}
          onRetry={() => void (query.isFetchNextPageError ? query.fetchNextPage() : query.refetch())}
          hasNextPage={query.hasNextPage ?? false}
          isFetchingNextPage={query.isFetchingNextPage}
          onLoadMore={() => void query.fetchNextPage()}
          emptyTitle={t("list.emptyTitle")}
          emptyDescription={t("list.emptyDesc")}
          emptyAction={
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setSearchInput("");
                setParams({ ...DEFAULT_LIST_PARAMS });
              }}
            >
              {t("list.resetFilters")}
            </Button>
          }
        />
      )}

      <ImageSearchDialog
        open={imgSearchOpen}
        onOpenChange={setImgSearchOpen}
        onResults={setImageResults}
      />

      {/* jump to page dialog */}
      <Dialog open={jumpOpen} onOpenChange={setJumpOpen} title={t("jump.title")}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            applyJump();
          }}
        >
          <label className="mb-1.5 block text-small font-medium text-muted-foreground" htmlFor="jump-page">
            {t("jump.pageLabel", {
              n: Math.max(1, Math.min(Math.round(Number(jumpValue)) || 1, maxJumpPage)),
              m: maxJumpPage,
            })}
          </label>
          <Input
            id="jump-page"
            type="number"
            min={1}
            max={maxJumpPage}
            value={jumpValue}
            onChange={(e) => setJumpValue(e.target.value)}
            onFocus={(e) => e.target.select()}
            autoFocus
          />
          <p className="mt-1.5 text-caption text-faint-foreground">
            {t("jump.rangeHint", { n: maxJumpPage })}
          </p>
          <div className="mt-3 flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setJumpOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button size="sm" type="submit">
              {t("jump.go")}
            </Button>
          </div>
        </form>
      </Dialog>

      {/* category sheet */}
      <CategorySheet open={catSheetOpen} onOpenChange={setCatSheetOpen} params={params} setParams={setParams} />

      {/* advanced sheet */}
      <AdvancedSearchSheet open={advSheetOpen} onOpenChange={setAdvSheetOpen} params={params} setParams={setParams} />
    </div>
  );
}

function SignInBanner() {
  const t = useT();
  const [dismissed, setDismissed] = useState(
    () => sessionStorage.getItem("exviewer.signin-banner") === "1",
  );
  const { data: signedIn } = useQuery({ queryKey: ["signin"], queryFn: checkSignIn });
  if (dismissed || signedIn) return null;
  return (
    <div className="flex shrink-0 items-center gap-3 border-b border-accent/20 bg-accent/5 px-4 py-2">
      <Info size={15} className="shrink-0 text-accent" />
      <p className="flex-1 text-small leading-5 text-muted-foreground">
        {t("list.banner.guest")}{" "}
        <Link to="/signin" className="font-medium text-accent hover:underline">
          {t("common.signIn")}
        </Link>{" "}
        {t("list.banner.suffix")}
      </p>
      <button
        aria-label={t("common.dismiss")}
        onClick={() => {
          setDismissed(true);
          sessionStorage.setItem("exviewer.signin-banner", "1");
        }}
        className="rounded p-0.5 text-faint-foreground hover:text-foreground"
      >
        <X size={14} />
      </button>
    </div>
  );
}
