import { loadMock } from "../api/transport";
import type { BlacklistEntry, GalleryInfo, HistoryEntry, NetworkConfig, QuickSearch, TagDbStatus, TagSuggestion } from "../types";

const HISTORY_PAGE_SIZE = 50;

export async function getHistory(page: number): Promise<HistoryEntry[]> {
  return (await loadMock()).getHistoryEntries()
    .slice(page * HISTORY_PAGE_SIZE, (page + 1) * HISTORY_PAGE_SIZE)
    .map((h) => ({ ...h, category: "", pages: 0 }));
}

export async function recordHistory(gallery: GalleryInfo): Promise<void> {
  (await loadMock()).pushHistory(gallery);
}

export async function removeHistory(gid: number): Promise<void> {
  (await loadMock()).removeHistory(gid);
}

export async function clearHistory(): Promise<void> {
  (await loadMock()).clearHistory();
}

const QUICK_KEY = "exviewer.quicksearch";

function loadQuickSearches(): QuickSearch[] {
  try {
    const raw = JSON.parse(localStorage.getItem(QUICK_KEY) ?? "[]") as unknown;
    if (!Array.isArray(raw)) return [];
    // Legacy format stored plain keyword strings.
    return raw.map((item, i) =>
      typeof item === "string"
        ? {
            id: -(i + 1),
            name: item,
            keyword: item,
            categories: null,
            minRating: -1,
            minPages: -1,
            maxPages: -1,
            time: 0,
          }
        : (item as QuickSearch),
    );
  } catch {
    return [];
  }
}

function saveQuickSearches(entries: QuickSearch[]) {
  localStorage.setItem(QUICK_KEY, JSON.stringify(entries));
}

export async function quickSearchList(): Promise<QuickSearch[]> {
  return loadQuickSearches();
}

export async function createQuickSearch(entry: {
  name: string;
  keyword: string;
  categories?: string | null;
  minRating?: number | null;
  minPages?: number | null;
  maxPages?: number | null;
}): Promise<QuickSearch> {
  const list = loadQuickSearches();
  const created: QuickSearch = {
    id: Date.now(),
    name: entry.name,
    keyword: entry.keyword,
    categories: entry.categories ?? null,
    minRating: entry.minRating ?? -1,
    minPages: entry.minPages ?? -1,
    maxPages: entry.maxPages ?? -1,
    time: Date.now(),
  };
  saveQuickSearches([...list, created]);
  return created;
}

export async function deleteQuickSearch(id: number): Promise<void> {
  saveQuickSearches(loadQuickSearches().filter((q) => q.id !== id));
}

const BLACKLIST_KEY = "exviewer.blacklist";

function loadBlacklist(): BlacklistEntry[] {
  try {
    return JSON.parse(localStorage.getItem(BLACKLIST_KEY) ?? "[]") as BlacklistEntry[];
  } catch {
    return [];
  }
}

export async function blacklistList(): Promise<BlacklistEntry[]> {
  return loadBlacklist();
}

export async function blacklistCreate(type: string, content: string): Promise<BlacklistEntry> {
  const entry: BlacklistEntry = { id: Date.now(), type, content, time: Date.now() };
  localStorage.setItem(BLACKLIST_KEY, JSON.stringify([...loadBlacklist(), entry]));
  return entry;
}

export async function blacklistDelete(id: number): Promise<void> {
  localStorage.setItem(BLACKLIST_KEY, JSON.stringify(loadBlacklist().filter((b) => b.id !== id)));
}

const DEFAULT_NETWORK_CONFIG: NetworkConfig = {
  site: "e",
  proxyType: "none",
  proxyHost: "",
  proxyPort: 0,
  dohEnabled: false,
  hosts: {},
  imageCacheSizeMb: 512,
  thumbResolution: 0,
  imageResolution: "a",
  fixThumbUrl: true,
};

const NETWORK_KEY = "exviewer.network";

export async function getNetworkConfig(): Promise<NetworkConfig> {
  try {
    const raw = localStorage.getItem(NETWORK_KEY);
    return { ...DEFAULT_NETWORK_CONFIG, ...(raw ? (JSON.parse(raw) as Partial<NetworkConfig>) : {}) };
  } catch {
    return { ...DEFAULT_NETWORK_CONFIG };
  }

}

export async function setNetworkConfig(config: NetworkConfig): Promise<boolean> {
  localStorage.setItem(NETWORK_KEY, JSON.stringify(config));
  return false;
}

const MOCK_TAG_SUGGESTIONS: TagSuggestion[] = [
  { namespace: "female", name: "sole_female", translated: "唯一女性" },
  { namespace: "female", name: "blowjob", translated: "口交" },
  { namespace: "female", name: "big_breasts", translated: "巨乳" },
  { namespace: "male", name: "yaoi", translated: "耽美" },
  { namespace: "male", name: "shotacon", translated: "正太" },
  { namespace: "language", name: "chinese", translated: "中文" },
  { namespace: "artist", name: "shindol", translated: "新堂エル" },
  { namespace: "parody", name: "touhou_project", translated: "东方Project" },
];

function startsWithCi(haystack: string, needle: string): boolean {
  return needle.length > 0 && haystack.toLowerCase().startsWith(needle.toLowerCase());
}

function mockSuggest(query: string): TagSuggestion[] {
  const q = query.trim();
  if (!q) return [];
  const out: TagSuggestion[] = [];
  const colon = q.indexOf(":");
  if (colon >= 0) {
    const nsQuery = q.slice(0, colon);
    const partial = q.slice(colon + 1);
    for (const t of MOCK_TAG_SUGGESTIONS) {
      if (out.length >= 10) break;
      if (!startsWithCi(t.namespace, nsQuery)) continue;
      if (!partial || startsWithCi(t.name, partial) || startsWithCi(t.translated, partial)) {
        out.push(t);
      }
    }
  } else {
    for (const t of MOCK_TAG_SUGGESTIONS) {
      if (out.length >= 10) break;
      if (startsWithCi(t.name, q) || startsWithCi(t.translated, q)) out.push(t);
    }
  }
  return out;
}

export async function suggestTags(query: string): Promise<TagSuggestion[]> {
  return mockSuggest(query);
}

export async function tagDbStatus(): Promise<TagDbStatus> {
  return { available: true, version: "mock", entryCount: MOCK_TAG_SUGGESTIONS.length };
}

export async function translateTags(tags: Array<{ namespace: string; name: string }>): Promise<Array<string | null>> {
  return (await loadMock()).mock.translateTags(tags);
}
