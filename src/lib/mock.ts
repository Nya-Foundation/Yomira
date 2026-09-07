import { demoCover, demoPage } from "./mock/artwork";
import type {
  ArchiverInfo,
  ArchiveResolution,
  Comment,
  DownloadEntry,
  DownloadedGallery,
  DownloadState,
  GalleryDetail,
  GalleryInfo,
  GalleryListResponse,
  ListParams,
  NewsItem,
  PageData,
  PreviewSet,
  Quota,
  TagGroup,
  TorrentInfo,
  ToplistTimespan,
  FavoriteItem,
} from "./types";
import { CATEGORIES } from "./types";

import { ApiError } from "./api/errors";

/* ---------- deterministic pseudo-random ---------- */

function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function delay(min = 180, max = 450) {
  const ms = min + Math.random() * (max - min);
  return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

/* ---------- data pools ---------- */

const TITLE_A = [
  "Midnight", "Silent", "Crimson", "Azure", "Golden", "Hidden", "Eternal",
  "Fleeting", "Velvet", "Paper", "Crystal", "Burning", "Frozen", "Hollow",
  "Radiant", "Broken", "Little", "Wild", "Quiet", "Nameless",
];
const TITLE_B = [
  "Garden", "Circuit", "Lantern", "Serenade", "Compass", "Orchard", "Mirror",
  "Harbor", "Signal", "Ember", "Canopy", "Drift", "Paradox", "Ribbon",
  "Static", "Bloom", "Voyage", "Chorus", "Fable", "Archive",
];
const TITLE_C = [
  "of the Lost Season", "After the Rain", "in Bloom", "Revisited",
  "- Side Story", "Vol. 2", "(Anthology)", "under Starlight", ": Complete",
  "of Yesterday", "- Final Chapter",
];
const JPN_A = ["Yoru no", "Mugen no", "Hikari no", "Kaze no", "Tsuki no", "Yume no", "Hana no"];
const JPN_B = ["Niwa", "Tabi", "Uta", "Kioku", "Machi", "Hoshi", "Kago"];
const UPLOADERS = [
  "aki_translations", "redcomet", "mangabox", "sakura_scans", "nightowl",
  "paper_crane", "mono_loader", "pixeldance", "kansai_kat", "archive_fan",
  "dailydoujin", "wanderer_x", "cosmo_db", "lilith_works", "greyfield",
];
const LANGUAGES = ["English", "Japanese", "Chinese", "Korean", "Spanish", "-"];
const NAMESPACES = ["artist", "group", "female", "male", "language", "parody", "character", "misc"];
const TAGS: Record<string, string[]> = {
  artist: ["aoki rin", "benimura", "chitose yuu", "dokuro-san", "emi tanaka", "fujii kaze"],
  group: ["studio moss", "paper works", "night radio", "anko club", "delta loop"],
  female: ["sole female", "schoolgirl uniform", "stockings", "glasses", "twintails", "elf", "maid"],
  male: ["sole male", "schoolboy uniform", "glasses", "dilf"],
  language: ["english", "japanese", "translated", "speechless"],
  parody: ["original", "galaxy express", "witch academy", "blade chronicle"],
  character: ["mira", "sana", "yuuki", "riko", "professor elm"],
  misc: ["full color", "multi-work series", "story arc", "doujinshi", "uncensored"],
};
const COMMENTERS = ["archive_rat", "page_turner", "quiet_reader", "melon_bread", "exv_fan", "thumbnail enjoyer"];
const nowSec = Math.floor(Date.now() / 1000);

/** Rich comment bodies mirroring e-hentai raw HTML: <br> line breaks, an <img>
 * emote, entities, [url] BBCode and ">green quote" lines. */
const RICH_COMMENTS: { text: string; lastEdited?: number }[] = [
  {
    text:
      "Fantastic art, the composition on every page is immaculate.<br>" +
      "The lighting in chapter 2 is on another level.",
  },
  {
    text:
      'Check the artist\'s other works here: [url=https://example.org/shimura0404]shimura0404\'s page[/url]<br>' +
      "Their older stuff is rougher but worth a look.",
  },
  {
    text:
      "This panel <img src=\"https://ehgt.org/v/emote/emote12.gif\"> made my whole day\n" +
      ">truly a masterpiece\n" +
      "Instantly added to favorites.",
  },
  {
    text:
      "Translations are rough in places but the story carries it. &amp; the typesetting is clean.",
    lastEdited: nowSec - 86400 * 12,
  },
  {
    text: "That ending though. Sequel when?\n[img]https://ehgt.org/v/emote/emote37.gif[/img]",
  },
  { text: "Underrated gem, more people should see this." },
  { text: "Scans are a bit muddy in the middle chapters but very readable overall." },
];

/** First page of comments embedded in the detail response; later pages come
 * from `getComments` (page is 1-based, page 1 = the detail page). */
const COMMENT_PAGE_SIZE = 5;

/* Votes and locally posted comments in the browser demo. Votes accumulate as
 * deltas so the score returned by `voteComment` matches what a refetch shows. */
const commentVoteDeltas = new Map<string, number>();
const postedComments = new Map<number, Comment[]>();

/** Deterministic base score of a pool comment (mirrors the draw order in
 * galleryComments: author, score, time). Ids >= 1000 are posted comments. */
function commentBaseScore(gid: number, id: number): number {
  if (id >= 1000) return 0;
  const rnd = mulberry32(gid * 17 + 3);
  let score = 0;
  for (let j = 0; j <= id; j++) {
    rnd(); // author
    score = Math.floor(rnd() * 41) - 10;
    rnd(); // time
  }
  return score;
}

function galleryComments(gid: number, page = 0): Comment[] {
  const rnd = mulberry32(gid * 17 + 3);
  const count = 5 + Math.floor(rnd() * (RICH_COMMENTS.length - 4));
  const start = page <= 1 ? 0 : (page - 1) * COMMENT_PAGE_SIZE;
  const pool = RICH_COMMENTS.slice(start, start + COMMENT_PAGE_SIZE).map((c, i) => ({
    id: start + i,
    author: COMMENTERS[Math.floor(rnd() * COMMENTERS.length)],
    score:
      Math.floor(rnd() * 41) -
      10 +
      (commentVoteDeltas.get(`${gid}:${start + i}`) ?? 0),
    time: nowSec - Math.floor(rnd() * 86400 * 300),
    lastEdited: c.lastEdited ?? 0,
    text: c.text,
  })).filter((_, i) => start + i < count);
  // locally posted comments ride along on the last pool page
  if (start + COMMENT_PAGE_SIZE >= RICH_COMMENTS.length) {
    pool.push(...(postedComments.get(gid) ?? []));
  }
  return pool;
}

const commentPoolSize = (gid: number) => {
  const rnd = mulberry32(gid * 17 + 3);
  return 5 + Math.floor(rnd() * (RICH_COMMENTS.length - 4)) + (postedComments.get(gid)?.length ?? 0);
};

/* ---------- gallery generation ---------- */

const TOTAL_GALLERIES = 640;
const PAGE_SIZE = 25;

function makeGallery(gid: number): GalleryInfo {
  const rnd = mulberry32(gid);
  const pick = <T>(arr: readonly T[]) => arr[Math.floor(rnd() * arr.length)];
  const category = pick(CATEGORIES);
  const title = `${pick(TITLE_A)} ${pick(TITLE_B)} ${rnd() > 0.4 ? pick(TITLE_C) : ""}`.trim();
  const titleJpn =
    rnd() > 0.35 ? `${pick(JPN_A)} ${pick(JPN_B)}${rnd() > 0.5 ? " no Sho" : ""}` : null;
  const pages = Math.floor(rnd() * 380) + 8;
  const rating = Math.round((1 + rnd() * 4) * 10) / 10;
  return {
    gid,
    token: (gid * 7 + 100000).toString(36),
    title,
    titleJpn,
    category,
    uploader: pick(UPLOADERS),
    posted: nowSec - Math.floor(rnd() * 86400 * 1800),
    pages,
    rating,
    ratedCount: Math.floor(rnd() * 2400) + 5,
    favorited: Math.floor(rnd() * 5200) + 2,
    language: pick(LANGUAGES),
    thumb: demoCover(gid, 360, 480, title.split(" ").slice(0, 2).join(" ")),
    size: pages * Math.floor(rnd() * 900000 + 300000),
    torrentCount: rnd() > 0.55 ? Math.floor(rnd() * 3) : 0,
    fileCount: pages,
  };
}

const galleryCache = new Map<number, GalleryInfo>();

function getGallery(gid: number): GalleryInfo {
  let g = galleryCache.get(gid);
  if (!g) {
    g = makeGallery(gid);
    galleryCache.set(gid, g);
  }
  return g;
}

function allGalleries(): GalleryInfo[] {
  return Array.from({ length: TOTAL_GALLERIES }, (_, i) => getGallery(1000001 + i * 13));
}

/* ---------- search filtering ---------- */

function matchesSearch(g: GalleryInfo, query: string, enableTags: boolean): boolean {
  const tokens = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return true;
  const tags = enableTags ? galleryTags(g.gid).flatMap((t) => t.tags) : [];
  return tokens.every((tok) => {
    const m = tok.match(/^(rating|pages)(:>=|:<=|:>|<|:)(.+)$/);
    if (m) {
      const value = parseFloat(m[3]);
      if (Number.isNaN(value)) return true;
      if (m[1] === "rating") {
        switch (m[2]) {
          case ">=": return g.rating >= value;
          case "<=": return g.rating <= value;
          case ">": return g.rating > value;
          case "<": return g.rating < value;
          default: return Math.abs(g.rating - value) < 0.01;
        }
      }
      switch (m[2]) {
        case ">=": return g.pages >= value;
        case "<=": return g.pages <= value;
        case ">": return g.pages > value;
        case "<": return g.pages < value;
        default: return g.pages === value;
      }
    }
    if (tok.startsWith("category:")) {
      return g.category.toLowerCase().replace(" ", "_") === tok.slice(9);
    }
    if (tok.startsWith("uploader:")) return g.uploader === tok.slice(9);
    if (tok.startsWith("tag:")) return tags.includes(tok.slice(4));
    return (
      g.title.toLowerCase().includes(tok) ||
      (g.titleJpn ?? "").toLowerCase().includes(tok) ||
      g.uploader.toLowerCase().includes(tok) ||
      tags.some((t) => t.includes(tok))
    );
  });
}

function applyParams(list: GalleryInfo[], p: ListParams): GalleryInfo[] {
  let out = list;
  if (p.categories && p.categories.length > 0) {
    out = out.filter((g) => p.categories!.includes(g.category));
  }
  if (p.minRating != null) out = out.filter((g) => g.rating >= p.minRating!);
  if (p.minPages != null) out = out.filter((g) => g.pages >= p.minPages!);
  if (p.maxPages != null) out = out.filter((g) => g.pages <= p.maxPages!);
  if (p.search) out = out.filter((g) => matchesSearch(g, p.search, p.enableTags));
  return out;
}

function pageOf(list: GalleryInfo[], page: number): GalleryListResponse {
  const start = page * PAGE_SIZE;
  return {
    galleries: list.slice(start, start + PAGE_SIZE),
    nextPage: start + PAGE_SIZE < list.length ? page + 1 : null,
    total: list.length,
    pages: list.length === 0 ? 0 : Math.ceil(list.length / PAGE_SIZE),
  };
}

/* ---------- detail data ---------- */

function galleryTags(gid: number): TagGroup[] {
  const rnd = mulberry32(gid * 31 + 7);
  return NAMESPACES.map((ns) => {
    const pool = TAGS[ns];
    const count = Math.floor(rnd() * (pool.length * 0.7)) + (ns === "misc" ? 1 : 0);
    const chosen = new Set<string>();
    for (let i = 0; i < count; i++) chosen.add(pool[Math.floor(rnd() * pool.length)]);
    return { namespace: ns, tags: [...chosen] };
  }).filter((t) => t.tags.length > 0);
}

function previewFor(gid: number, index: number) {
  const rnd = mulberry32(gid * 11 + index);
  const w = 300;
  const h = Math.floor(rnd() * 120) + 380;
  return { index, thumb: demoCover(gid * 1000 + index, w, h), width: w, height: h };
}

const detailCache = new Map<number, GalleryDetail>();

function getDetail(gid: number): GalleryDetail {
  let d = detailCache.get(gid);
  if (d) return d;
  const base = getGallery(gid);
  const total = base.pages;
  const previews = Array.from({ length: Math.min(total, 40) }, (_, i) => previewFor(gid, i));
  d = {
    ...base,
    tags: galleryTags(gid),
    comments: galleryComments(gid, 1),
    commentCount: commentPoolSize(gid),
    hasMoreComments: commentPoolSize(gid) > COMMENT_PAGE_SIZE,
    previewSet: { gid, previews, total },
    isFavorited: favoriteEntries().some((f) => f.gid === gid),
    favoriteSlot: favoriteEntries().find((f) => f.gid === gid)?.slot ?? -1,
    myRating: 0,
  };
  detailCache.set(gid, d);
  return d;
}

/* ---------- favorites / history (localStorage) ---------- */

interface FavEntry {
  gid: number;
  slot: number;
  note: string;
  time: number;
}

const FAV_KEY = "exviewer.favorites";
const HIST_KEY = "exviewer.history";

interface HistoryEntry {
  gid: number;
  token: string;
  title: string;
  thumb: string;
  time: number;
}

function favoriteEntries(): FavEntry[] {
  try {
    return JSON.parse(localStorage.getItem(FAV_KEY) ?? "[]") as FavEntry[];
  } catch {
    return [];
  }
}

function saveFavorites(entries: FavEntry[]) {
  localStorage.setItem(FAV_KEY, JSON.stringify(entries));
}

export function getHistoryEntries(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem(HIST_KEY) ?? "[]") as HistoryEntry[];
  } catch {
    return [];
  }
}

export function pushHistory(g: GalleryInfo) {
  const list = getHistoryEntries().filter((h) => h.gid !== g.gid);
  list.unshift({ gid: g.gid, token: g.token, title: g.title, thumb: g.thumb, time: Date.now() });
  localStorage.setItem(HIST_KEY, JSON.stringify(list.slice(0, 200)));
}

export function removeHistory(gid: number) {
  localStorage.setItem(HIST_KEY, JSON.stringify(getHistoryEntries().filter((h) => h.gid !== gid)));
}

export function clearHistory() {
  localStorage.removeItem(HIST_KEY);
}

/* ---------- downloads, news, quota, search history (localStorage) ---------- */

const DL_KEY = "exviewer.downloads";
const DL_SEED_KEY = "exviewer.downloads.seeded";
const DL_LABELS_KEY = "exviewer.download-labels";
const DL_ROOT_KEY = "exviewer.download-root";
const SEARCH_HIST_KEY = "exviewer.search-history";
const NEWS_PAGE_SIZE = 20;
const SEARCH_HIST_MAX = 20;

interface StoredDownload {
  gid: number;
  token: string;
  state: DownloadState;
  label: string | null;
  finishedPages: number;
  error: string | null;
  time: number;
}

function loadDownloads(): StoredDownload[] {
  try {
    return JSON.parse(localStorage.getItem(DL_KEY) ?? "[]") as StoredDownload[];
  } catch {
    return [];
  }
}

function saveDownloads(list: StoredDownload[]) {
  localStorage.setItem(DL_KEY, JSON.stringify(list));
}

function loadDownloadLabels(): string[] {
  try {
    return JSON.parse(localStorage.getItem(DL_LABELS_KEY) ?? "[]") as string[];
  } catch {
    return [];
  }
}

function loadSearchHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(SEARCH_HIST_KEY) ?? "[]") as string[];
  } catch {
    return [];
  }
}

function mockDownloadRoot(): string {
  return localStorage.getItem(DL_ROOT_KEY) ?? "C:\\Yomira\\Downloads";
}

function saveSearchHistory(list: string[]) {
  localStorage.setItem(SEARCH_HIST_KEY, JSON.stringify(list.slice(0, SEARCH_HIST_MAX)));
}

/** First run of the browser demo seeds a small queue so the manager has
 * something to show; after that the user owns the list. */
function seedDownloadsIfNeeded() {
  if (localStorage.getItem(DL_SEED_KEY)) return;
  localStorage.setItem(DL_SEED_KEY, "1");
  const pick = (n: number) => 1000001 + n * 13;
  saveDownloads([
    {
      gid: pick(3),
      token: getGallery(pick(3)).token,
      state: "downloading",
      label: "demo",
      finishedPages: 12,
      error: null,
      time: Date.now() - 3600_000,
    },
    {
      gid: pick(7),
      token: getGallery(pick(7)).token,
      state: "queued",
      label: null,
      finishedPages: 0,
      error: null,
      time: Date.now() - 1200_000,
    },
    {
      gid: pick(11),
      token: getGallery(pick(11)).token,
      state: "done",
      label: "demo",
      finishedPages: getGallery(pick(11)).pages,
      error: null,
      time: Date.now() - 86400_000,
    },
  ]);
  if (loadDownloadLabels().length === 0) {
    localStorage.setItem(DL_LABELS_KEY, JSON.stringify(["demo"]));
  }
}

const NEWS_POOL: NewsItem[] = [
  {
    title: "Server maintenance this weekend",
    date: new Date(Date.now() - 3600_000 * 5).toISOString(),
    body: "The main gallery servers will be down for maintenance on Saturday between 02:00 and 06:00 UTC. Browsing may be unavailable during this window.",
    link: "https://e-hentai.org/news.php",
  },
  {
    title: "New image server online",
    date: new Date(Date.now() - 86400_000 * 2).toISOString(),
    body: "A new image server has been added to the pool in the EU region. Users in Europe should see faster page loads.",
    link: "https://e-hentai.org/news.php",
  },
  {
    title: "Tag database updated",
    date: new Date(Date.now() - 86400_000 * 5).toISOString(),
    body: "The tag translation database has been regenerated from the latest dump. Power users can trigger a refresh from the app settings.",
    link: "https://e-hentai.org/news.php",
  },
  {
    title: "H@H client requirements raised",
    date: new Date(Date.now() - 86400_000 * 9).toISOString(),
    body: "Minimum trust and speed requirements for new H@H clients have been raised slightly to keep up with demand.",
    link: "https://e-hentai.org/news.php",
  },
  {
    title: "Scheduled downtime completed",
    date: new Date(Date.now() - 86400_000 * 14).toISOString(),
    body: "Last night's maintenance completed ahead of schedule. All services are back to normal.",
    link: "https://e-hentai.org/news.php",
  },
  {
    title: "Forum migration notes",
    date: new Date(Date.now() - 86400_000 * 21).toISOString(),
    body: "The forums have been migrated to new hardware. Old deep links may redirect for a while; update your bookmarks.",
    link: "https://e-hentai.org/news.php",
  },
];

/* ---------- public mock API ---------- */

/** EhViewer-style keys (`namespace:name`, spaces normalized to underscores)
 * for the small fixed translation dictionary used by the browser demo. */
const MOCK_TAG_TRANSLATIONS: Record<string, string> = {
  "female:yuri": "百合",
  "artist:shimura0404": "志摩三四",
  "misc:full_color": "全彩",
  "female:sole_female": "唯一女性",
  "female:schoolgirl_uniform": "校服",
  "female:stockings": "丝袜",
  "female:glasses": "眼镜",
  "female:twintails": "双马尾",
  "female:elf": "精灵",
  "female:maid": "女仆",
  "male:sole_male": "唯一男性",
  "language:chinese": "中文",
  "language:translated": "翻译",
  "language:speechless": "无对话",
  "misc:uncensored": "无修正",
  "misc:doujinshi": "同人志",
  "misc:multi_work_series": "系列作品",
  "artist:aoki_rin": "青木凛",
};

interface MockTagRef {
  namespace: string;
  name: string;
}

const SORTED_TOP: Record<ToplistTimespan, GalleryInfo[]> = {
  yesterday: [...allGalleries()].sort((a, b) => b.posted - a.posted),
  month: [...allGalleries()].sort((a, b) => b.rating - a.rating),
  year: [...allGalleries()].sort((a, b) => b.favorited - a.favorited),
  all: [...allGalleries()].sort(
    (a, b) => b.favorited * b.rating - a.favorited * a.rating,
  ),
};

const WATCHED_TAG = "female:sole_female";

export const mock = {
  async getGalleryList(
    mode: "home" | "popular" | "watched" | "toplist",
    p: ListParams,
    timespan: ToplistTimespan,
  ): Promise<GalleryListResponse> {
    await delay();
    let list = allGalleries();
    if (mode === "popular") list = SORTED_TOP.month;
    if (mode === "watched") {
      const tag = WATCHED_TAG.slice(6);
      list = list.filter((g) => galleryTags(g.gid).find((t) => t.namespace === "female")?.tags.includes(tag));
    }
    if (mode === "toplist") list = SORTED_TOP[timespan];
    return pageOf(applyParams(list, p), p.page);
  },

  async getGalleryDetail(gid: number): Promise<GalleryDetail> {
    await delay();
    return structuredClone(getDetail(gid));
  },

  async getPreviewSet(gid: number, index: number): Promise<PreviewSet> {
    await delay(120, 260);
    const total = getGallery(gid).pages;
    const start = index * 20;
    const previews = Array.from(
      { length: Math.min(20, total - start) },
      (_, i) => previewFor(gid, start + i),
    );
    return { gid, previews, total };
  },

  async getPageData(gid: number, index: number): Promise<PageData> {
    await delay(150, 500);
    // ~2% of pages fail with a 509-style error so the error card can be demoed
    if ((gid * 7919 + index * 31) % 97 === 13) {
      throw new ApiError(509, "Bandwidth limit exceeded (mock 509)", "image509");
    }
    const width = 1280;
    const height = 1810;
    return { index, url: demoPage(gid, index, width, height), width, height };
  },

  async getFavorites(
    page: number,
    slot: number | null,
    sort: "f" | "p" | null = null,
  ): Promise<{ items: FavoriteItem[]; nextPage: number | null }> {
    await delay();
    let entries = favoriteEntries();
    if (slot !== null) entries = entries.filter((e) => e.slot === slot);
    const items = entries.map((e) => ({
      ...getGallery(e.gid),
      favoriteSlot: e.slot,
      favoriteNote: e.note,
      favoritedTime: e.time,
    }));
    items.sort((a, b) => (sort === "p" ? b.posted - a.posted : b.favoritedTime - a.favoritedTime));
    const pageItems = items.slice(page * 25, page * 25 + 25);
    return { items: pageItems, nextPage: (page + 1) * 25 < items.length ? page + 1 : null };
  },

  async getFavoriteNote(gid: number): Promise<string> {
    await delay(60, 120);
    return favoriteEntries().find((e) => e.gid === gid)?.note ?? "";
  },

  async modifyFavoriteNote(gid: number, note: string): Promise<void> {
    await delay(100, 200);
    const entries = favoriteEntries();
    const hit = entries.find((e) => e.gid === gid);
    if (hit) {
      hit.note = note;
      saveFavorites(entries);
    }
  },

  async toggleFavorite(gid: number, slot: number): Promise<{ isFavorited: boolean; slot: number }> {
    await delay(120, 250);
    const entries = favoriteEntries();
    const existing = entries.findIndex((e) => e.gid === gid);
    if (existing >= 0) {
      entries.splice(existing, 1);
      saveFavorites(entries);
      return { isFavorited: false, slot: -1 };
    }
    entries.push({ gid, slot, note: "", time: Date.now() });
    saveFavorites(entries);
    return { isFavorited: true, slot };
  },

  async rateGallery(_gid: number, rating: number): Promise<{ rating: number }> {
    await delay(100, 200);
    return { rating };
  },

  async getComments(gid: number, page = 1): Promise<Comment[]> {
    await delay();
    return galleryComments(gid, page);
  },

  async voteComment(gid: number, commentId: number, vote: 1 | -1): Promise<number> {
    await delay(80, 160);
    const key = `${gid}:${commentId}`;
    const delta = (commentVoteDeltas.get(key) ?? 0) + vote;
    commentVoteDeltas.set(key, delta);
    return commentBaseScore(gid, commentId) + delta;
  },

  async sendComment(gid: number, text: string): Promise<void> {
    await delay(200, 400);
    const list = postedComments.get(gid) ?? [];
    list.push({
      id: 1000 + list.length,
      author: "you",
      score: 0,
      time: Math.floor(Date.now() / 1000),
      lastEdited: 0,
      text,
    });
    postedComments.set(gid, list);
  },

  async getTorrentList(gid: number): Promise<TorrentInfo[]> {
    await delay();
    const g = getGallery(gid);
    const rnd = mulberry32(gid * 13 + 1);
    return [
      {
        name: `${g.title} [English].torrent`,
        size: g.size,
        seeds: Math.floor(rnd() * 20) + 2,
        peers: Math.floor(rnd() * 8) + 1,
        downloads: Math.floor(rnd() * 900) + 40,
        url: `https://e-hentai.org/torrent/${gid}/${g.token}/english.torrent`,
      },
      {
        name: `${g.title} [Japanese].torrent`,
        size: g.size,
        seeds: Math.floor(rnd() * 12) + 1,
        peers: Math.floor(rnd() * 6) + 1,
        downloads: Math.floor(rnd() * 500) + 20,
        url: `https://e-hentai.org/torrent/${gid}/${g.token}/japanese.torrent`,
      },
    ];
  },

  async downloadTorrent(_gid: number, url: string): Promise<string> {
    await delay(300, 700);
    const name = url.split("/").pop() ?? "gallery.torrent";
    return `${mockDownloadRoot()}\\torrents\\${name}`;
  },

  async getArchiverInfo(gid: number): Promise<ArchiverInfo> {
    await delay();
    const g = getGallery(gid);
    return {
      action: "https://e-hentai.org/archiver.php",
      params: [
        ["gid", String(gid)],
        ["token", g.token],
        ["archiver_id", (gid * 7919).toString()],
      ],
      costText: "Download Cost: 10,000 credits. You currently have 427,020 credits.",
    };
  },

  async downloadArchive(gid: number, resolution: ArchiveResolution): Promise<string> {
    await delay(400, 900);
    return `${mockDownloadRoot()}\\${gid}${resolution === "org" ? "" : "-resample"}.zip`;
  },

  async translateTags(tags: MockTagRef[]): Promise<Array<string | null>> {
    await delay(60, 140);
    return tags.map(
      (tag) =>
        MOCK_TAG_TRANSLATIONS[`${tag.namespace}:${tag.name.replace(/ /g, "_")}`] ?? null,
    );
  },

  async checkSignIn(): Promise<boolean> {
    await delay(60, 120);
    return localStorage.getItem("exviewer.signed_in") === "1";
  },

  async signInWithWebview(): Promise<void> {
    await delay(1400, 1600);
    localStorage.setItem("exviewer.signed_in", "1");
  },

  async signInWithCookies(cookies: string): Promise<void> {
    await delay(1000, 1400);
    const required = ["ipb_member_id", "ipb_pass_hash"];
    const found = required.every((k) => cookies.includes(`${k}=`));
    if (!found) {
      throw new ApiError(400, "Could not find ipb_member_id / ipb_pass_hash in the pasted cookies");
    }
    localStorage.setItem("exviewer.signed_in", "1");
  },

  async signOut(): Promise<void> {
    await delay(80, 150);
    localStorage.removeItem("exviewer.signed_in");
  },

  async getImage(url: string): Promise<string> {
    await delay(80, 200);
    return url;
  },

  // --- downloads ---

  async getDownloadedGallery(gid: number, token: string): Promise<DownloadedGallery | null> {
    const saved = loadDownloads().find((d) => d.gid === gid && d.token === token);
    if (!saved) return null;
    const g = getGallery(gid);
    return { gid, token, title: g.title, pages: g.pages,
      availablePages: Array.from({ length: Math.min(g.pages, saved.finishedPages) }, (_, i) => i) };
  },

  async getDownloadedPage(gid: number, token: string, index: number): Promise<PageData> {
    const saved = await this.getDownloadedGallery(gid, token);
    if (!saved?.availablePages.includes(index)) {
      throw new ApiError(404, "This page is not available in the download folder.", "pageUnavailable");
    }
    return { index, url: demoPage(gid, index, 1280, 1810), width: 1280, height: 1810 };
  },

  async listDownloads(): Promise<DownloadEntry[]> {
    await delay();
    seedDownloadsIfNeeded();
    const root = mockDownloadRoot();
    return loadDownloads().map((s) => {
      const g = getGallery(s.gid);
      return {
        gid: s.gid,
        token: s.token,
        title: g.title,
        thumb: g.thumb,
        category: g.category,
        pages: g.pages,
        state: s.state,
        label: s.label,
        dirname: s.state === "done" || s.state === "downloading" ? `${s.gid}` : null,
        finishedPages: s.finishedPages,
        localThumb: s.finishedPages > 0 ? demoPage(s.gid, 0, 160, 226) : null,
        dirPath: s.state === "done" ? `${root}\\${s.gid}` : null,
        error: s.error,
        time: s.time,
      };
    });
  },

  async getDownloadRoot(): Promise<string> {
    await delay(60, 120);
    return mockDownloadRoot();
  },

  async setDownloadRoot(path: string): Promise<void> {
    await delay(100, 200);
    localStorage.setItem(DL_ROOT_KEY, path);
  },

  async addDownload(gid: number, token: string, label: string | null = null): Promise<void> {
    await delay(120, 250);
    seedDownloadsIfNeeded();
    const list = loadDownloads();
    const existing = list.find((d) => d.gid === gid);
    if (existing) {
      existing.state = "queued";
      existing.error = null;
      if (label) existing.label = label;
    } else {
      list.unshift({ gid, token, state: "queued", label, finishedPages: 0, error: null, time: Date.now() });
    }
    saveDownloads(list);
  },

  async removeDownload(gid: number, _deleteFiles: boolean): Promise<void> {
    await delay(100, 200);
    saveDownloads(loadDownloads().filter((d) => d.gid !== gid));
  },

  async startDownload(gid: number): Promise<void> {
    await delay(100, 200);
    const list = loadDownloads();
    const hit = list.find((d) => d.gid === gid);
    if (hit) {
      hit.state = "downloading";
      hit.error = null;
      saveDownloads(list);
    }
  },

  async pauseDownload(gid: number): Promise<void> {
    await delay(100, 200);
    const list = loadDownloads();
    const hit = list.find((d) => d.gid === gid);
    if (hit) {
      hit.state = "paused";
      saveDownloads(list);
    }
  },

  async downloadLabels(): Promise<string[]> {
    await delay(60, 120);
    seedDownloadsIfNeeded();
    return loadDownloadLabels();
  },

  async addDownloadLabel(name: string): Promise<void> {
    await delay(80, 160);
    const labels = loadDownloadLabels();
    if (!labels.includes(name)) {
      localStorage.setItem(DL_LABELS_KEY, JSON.stringify([...labels, name]));
    }
  },

  async deleteDownloadLabel(name: string): Promise<void> {
    await delay(80, 160);
    localStorage.setItem(DL_LABELS_KEY, JSON.stringify(loadDownloadLabels().filter((l) => l !== name)));
    saveDownloads(
      loadDownloads().map((d) => (d.label === name ? { ...d, label: null } : d)),
    );
  },

  async setDownloadLabel(gid: number, label: string | null): Promise<void> {
    await delay(80, 160);
    const list = loadDownloads();
    const hit = list.find((d) => d.gid === gid);
    if (hit) {
      hit.label = label;
      saveDownloads(list);
    }
  },

  // --- extras ---

  async imageLookup(): Promise<GalleryListResponse> {
    await delay(500, 900);
    return pageOf(allGalleries(), 0);
  },

  async getNews(page: number): Promise<NewsItem[]> {
    await delay();
    return NEWS_POOL.slice(page * NEWS_PAGE_SIZE, (page + 1) * NEWS_PAGE_SIZE);
  },

  async getQuota(): Promise<Quota> {
    await delay(60, 120);
    return {
      imagesUsed: 137,
      imagesTotal: 5000,
      imageResetSeconds: 5 * 3600 - 732,
      searchesUsed: 3,
      searchesTotal: -1,
      searchResetSeconds: null,
    };
  },

  async searchHistory(): Promise<string[]> {
    await delay(60, 120);
    return loadSearchHistory();
  },

  async recordSearch(term: string): Promise<void> {
    const next = [term, ...loadSearchHistory().filter((x) => x !== term)];
    saveSearchHistory(next);
  },

  async clearSearchHistory(): Promise<void> {
    localStorage.removeItem(SEARCH_HIST_KEY);
  },

  async deleteSearchTerm(term: string): Promise<void> {
    saveSearchHistory(loadSearchHistory().filter((x) => x !== term));
  },
};
