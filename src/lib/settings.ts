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
 * The theme, mirrored where it can be read synchronously. `loadPersisted`
 * answers over async IPC in the app, so without this the dark-first palette
 * paints first and a light-theme reader sees it flip on every cold start.
 */
const THEME_KEY = "exviewer.theme";

function rememberTheme(theme: Theme) {
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    /* unavailable — the async load still applies the theme, just later */
  }
}

/** Apply the last known theme. Call before mounting, not from a component. */
export function applyRememberedTheme() {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    if (stored === "light" || stored === "dark") applyTheme(stored);
  } catch {
    /* unavailable — fall back to the dark-first default */
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
    rememberTheme(settings.theme);
  },
  update: (patch) => {
    const settings = { ...get().settings, ...patch };
    set({ settings });
    void persist(settings);
    rememberTheme(settings.theme);
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
