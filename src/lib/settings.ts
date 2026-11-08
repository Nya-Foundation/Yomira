import { Store } from "@tauri-apps/plugin-store";
import { create } from "zustand";
import { isTauri } from "./api/transport";

export type Theme = "dark" | "light";
export type Density = "comfortable" | "compact";
export type ReaderMode = "paged" | "continuous";
/** UI language preference. "system" follows the OS/webview locale. */
export type LanguageSetting = "system" | "en" | "zh-CN";

export interface AppSettings {
  theme: Theme;
  density: Density;
  language: LanguageSetting;
  /** UI scale as a multiplier: 0.75-2. Applied via root font-size (rem scale). */
  displayScale: number;
  galleryView: "thumbnail" | "detail";
  readerDirection: "ltr" | "rtl";
  readerMode: ReaderMode;
  /** "page" fits the whole page in view; "width" fills the width and scrolls. */
  readerFit: "page" | "width";
  readerPreloadCount: number;
}

const DEFAULTS: AppSettings = {
  theme: "dark",
  density: "comfortable",
  language: "system",
  displayScale: 1,
  galleryView: "thumbnail",
  readerDirection: "ltr",
  readerMode: "paged",
  readerFit: "page",
  readerPreloadCount: 3,
};

const LS_KEY = "exviewer.settings";

/**
 * Everything the first paint needs, mirrored where it can be read
 * synchronously. The persisted copy answers over IPC, and an Android cold
 * start drops that first response outright — so without this the dark-first
 * palette at the default scale paints and then jumps, or waits on a reply
 * that never comes.
 */
const PAINT_KEY = "exviewer.paint";
const LEGACY_THEME_KEY = "exviewer.theme";

type PaintSettings = Pick<AppSettings, "theme" | "density" | "displayScale">;

function rememberPaint({ theme, density, displayScale }: AppSettings) {
  try {
    localStorage.setItem(PAINT_KEY, JSON.stringify({ theme, density, displayScale }));
  } catch {
    /* unavailable — the async load still applies these, just later */
  }
}

/** Apply the last known presentation. Call before mounting, not from a component. */
export function applyRememberedPaint() {
  try {
    const raw = localStorage.getItem(PAINT_KEY);
    const paint = raw ? (JSON.parse(raw) as Partial<PaintSettings>) : null;
    const theme = paint?.theme ?? localStorage.getItem(LEGACY_THEME_KEY);
    if (theme === "light" || theme === "dark") applyTheme(theme);
    if (paint?.density === "compact" || paint?.density === "comfortable") applyDensity(paint.density);
    if (typeof paint?.displayScale === "number" && paint.displayScale > 0) {
      applyDisplayScale(paint.displayScale);
    }
  } catch {
    /* unavailable — fall back to the dark-first defaults */
  }
}

let store: Store | null = null;

async function loadPersisted(): Promise<Partial<AppSettings>> {
  if (isTauri()) {
    try {
      store ??= await Store.load("settings.json");
      const raw = await store.get<Record<string, unknown>>("settings");
      return (raw as Partial<AppSettings>) ?? {};
    } catch {
      return {};
    }
  }
  try {
    const raw = localStorage.getItem(LS_KEY);
    return raw ? (JSON.parse(raw) as Partial<AppSettings>) : {};
  } catch {
    return {};
  }
}

async function persist(settings: AppSettings) {
  if (isTauri()) {
    try {
      store ??= await Store.load("settings.json");
      await store.set("settings", settings);
      await store.save();
      return;
    } catch {
      /* fall through to localStorage */
    }
  }
  localStorage.setItem(LS_KEY, JSON.stringify(settings));
}

interface SettingsState {
  settings: AppSettings;
  loaded: boolean;
  init: () => Promise<void>;
  update: (patch: Partial<AppSettings>) => void;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: DEFAULTS,
  loaded: false,
  init: async () => {
    if (get().loaded) return;
    const persisted = await loadPersisted();
    const settings = { ...DEFAULTS, ...persisted };
    set({ settings, loaded: true });
    rememberPaint(settings);
  },
  update: (patch) => {
    const settings = { ...get().settings, ...patch };
    set({ settings });
    void persist(settings);
    rememberPaint(settings);
    applyTheme(settings.theme);
    applyDensity(settings.density);
    applyDisplayScale(settings.displayScale);
  },
}));

export function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("light", theme === "light");
}

export function applyDensity(density: Density) {
  document.documentElement.classList.toggle("dense", density === "compact");
}

/**
 * Scales the whole UI by scaling the root font-size, which every Tailwind
 * spacing/type utility derives from (rem-based). px-based values (border
 * widths, JS-computed reader geometry, TanStack Virtual measurements) stay
 * unscaled, which is safe: the virtualizer measures through ResizeObserver in
 * the same coordinate system it renders in, so it self-corrects.
 *
 * CSS `zoom` was tried first and rejected: WebView2 applies legacy zoom
 * semantics where `vh`/`vw` keep their pre-zoom size, so any `h-screen`/
 * `w-full` container overflows the visual viewport and the bottom/right of
 * the layout is clipped (e.g. the sidebar's settings/account buttons became
 * unreachable at zoom 1.25). Root font-size scaling keeps viewport units
 * exact and cannot clip.
 */
export function applyDisplayScale(scale: number) {
  document.documentElement.style.fontSize = scale === 1 ? "" : `${14 * scale}px`;
}
