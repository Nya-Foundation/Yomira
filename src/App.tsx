import { lazy, Suspense, useEffect, useState } from "react";
import { Route, Routes, useLocation } from "react-router-dom";
import { Sidebar } from "./components/layout/sidebar";
import { Header } from "./components/layout/header";
import { CommandPalette } from "./components/layout/command-palette";
import { ShortcutsDialog } from "./components/layout/shortcuts-dialog";
import { ToastViewport } from "./components/ui/toast";
import { Spinner } from "./components/ui/spinner";

const GalleryListPage = lazy(() =>
  import("./pages/GalleryListPage").then((m) => ({ default: m.GalleryListPage })),
);
const GalleryDetailPage = lazy(() =>
  import("./pages/GalleryDetailPage").then((m) => ({ default: m.GalleryDetailPage })),
);
const ReaderPage = lazy(() => import("./pages/ReaderPage").then((m) => ({ default: m.ReaderPage })));
const FavoritesPage = lazy(() =>
  import("./pages/FavoritesPage").then((m) => ({ default: m.FavoritesPage })),
);
const HistoryPage = lazy(() =>
  import("./pages/HistoryPage").then((m) => ({ default: m.HistoryPage })),
);
const SettingsPage = lazy(() =>
  import("./pages/SettingsPage").then((m) => ({ default: m.SettingsPage })),
);
const SignInPage = lazy(() => import("./pages/SignInPage").then((m) => ({ default: m.SignInPage })));
const DownloadsPage = lazy(() =>
  import("./pages/DownloadsPage").then((m) => ({ default: m.DownloadsPage })),
);
const NewsPage = lazy(() => import("./pages/NewsPage").then((m) => ({ default: m.NewsPage })));
import { useSettings, applyTheme, applyDensity, applyDisplayScale } from "./lib/settings";
import { I18nProvider, useT } from "./lib/i18n";
import { isEditableTarget } from "./lib/utils";
import { useZoomShortcuts } from "./lib/use-zoom-shortcuts";

export default function App() {
  const t = useT();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const { pathname, key: locationKey } = useLocation();
  const isReader = pathname.startsWith("/read/");

  const initSettings = useSettings((s) => s.init);
  const loaded = useSettings((s) => s.loaded);
  useZoomShortcuts(loaded);
  const theme = useSettings((s) => s.settings.theme);
  const density = useSettings((s) => s.settings.density);
  const displayScale = useSettings((s) => s.settings.displayScale);

  useEffect(() => {
    void initSettings();
  }, [initSettings]);

  useEffect(() => {
    if (loaded) {
      applyTheme(theme);
      applyDensity(density);
      applyDisplayScale(displayScale);
    }
  }, [loaded, theme, density, displayScale]);

  // global keyboard shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (document.querySelector("dialog[open]")) return;
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
        return;
      }
      if (isEditableTarget(e.target)) return;
      if (e.key === "/") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("yomira:focus-search"));
      } else if (e.key === "?") {
        e.preventDefault();
        setShortcutsOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Hold first paint until persisted settings (theme, density, display
  // scale) are loaded so the saved scale applies without a flash.
  if (!loaded) return null;

  return (
    <I18nProvider>
      <a href="#main-content" className="skip-link">{t("nav.skip")}</a>
      <div className="app-shell flex h-dvh w-full overflow-hidden bg-background">
        {!isReader && <Sidebar />}
        <div className="flex min-w-0 flex-1 flex-col">
        {!isReader && <Header onOpenPalette={() => setPaletteOpen(true)} />}
        <main id="main-content" tabIndex={-1} className={`min-h-0 flex-1 overflow-hidden ${isReader ? "" : "app-main"}`}>
          <Suspense
            fallback={
              <div className="flex h-full items-center justify-center">
                <Spinner size={24} />
              </div>
            }
          >
          <Routes>
            <Route path="/" element={<GalleryListPage key={locationKey} mode="home" />} />
            <Route path="/popular" element={<GalleryListPage key={locationKey} mode="popular" />} />
            <Route path="/watched" element={<GalleryListPage key={locationKey} mode="watched" />} />
            <Route path="/toplist" element={<GalleryListPage key={locationKey} mode="toplist" />} />
            <Route path="/favorites" element={<FavoritesPage />} />
            <Route path="/downloads" element={<DownloadsPage />} />
            <Route path="/news" element={<NewsPage />} />
            <Route path="/history" element={<HistoryPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/signin" element={<SignInPage />} />
            <Route path="/gallery/:gid/:token" element={<GalleryDetailPage />} />
            <Route path="/read/:gid/:token" element={<ReaderPage />} />
            <Route path="*" element={<GalleryListPage key="home" mode="home" />} />
          </Routes>
          </Suspense>
        </main>
      </div>
        <CommandPalette open={paletteOpen} onOpenChange={setPaletteOpen} />
        <ShortcutsDialog open={shortcutsOpen} onOpenChange={setShortcutsOpen} />
        <ToastViewport />
      </div>
    </I18nProvider>
  );
}
