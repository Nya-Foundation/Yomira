import { useLocation, useNavigate } from "react-router-dom";
import { Search, Command } from "lucide-react";
import { cn } from "../../lib/utils";
import { useT, type TFunction, type TKey } from "../../lib/i18n";
import { NavigationControls } from "./navigation-controls";

export interface RouteMeta {
  path: string;
  labelKey: TKey;
}

export const ROUTE_META: RouteMeta[] = [
  { path: "/", labelKey: "nav.home" },
  { path: "/popular", labelKey: "nav.popular" },
  { path: "/watched", labelKey: "nav.watched" },
  { path: "/toplist", labelKey: "nav.toplist" },
  { path: "/favorites", labelKey: "nav.favorites" },
  { path: "/downloads", labelKey: "nav.downloads" },
  { path: "/history", labelKey: "nav.history" },
  { path: "/news", labelKey: "nav.news" },
  { path: "/settings", labelKey: "nav.settings" },
  { path: "/signin", labelKey: "nav.signin" },
];

function routeLabel(pathname: string, t: TFunction): string {
  const meta = ROUTE_META.find((r) => r.path === pathname);
  if (meta) return t(meta.labelKey);
  if (pathname.startsWith("/gallery/")) return t("nav.gallery");
  if (pathname.startsWith("/read/")) return t("nav.reader");
  return t("nav.app");
}

const LIST_ROUTES = ["/", "/popular", "/watched", "/toplist"];

export function Header({ onOpenPalette }: { onOpenPalette: () => void }) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const t = useT();
  const title = routeLabel(pathname, t);
  const isList = LIST_ROUTES.some((r) => pathname === r);

  return (
    <header className="flex h-16 shrink-0 items-center gap-1.5 border-b border-border bg-background px-2 sm:gap-3 sm:px-5">
      <NavigationControls />
      <h1 className="min-w-0 truncate text-title font-semibold tracking-tight">{title}</h1>
      <div className="flex-1" />
      {!isList && (
        <button
          onClick={() => navigate("/", { state: { focusSearch: true } })}
          aria-label={t("list.searchAria")}
          className={cn(
            "flex h-9 w-9 sm:w-48 items-center gap-2 rounded-lg border border-border bg-surface px-2.5",
            "text-small text-faint-foreground transition-colors duration-150 hover:border-border-strong hover:text-muted-foreground",
          )}
        >
          <Search size={14} strokeWidth={1.75} />
          <span className="hidden flex-1 text-left sm:block">{t("header.searchPlaceholder")}</span>
          <kbd className="hidden sm:block rounded border border-border bg-surface-2 px-1 font-mono text-[10px] text-faint-foreground">
            /
          </kbd>
        </button>
      )}
      <button
        onClick={onOpenPalette}
        aria-label={t("header.jumpTo")}
        className={cn(
          "flex h-9 items-center gap-2 rounded-lg border border-border bg-surface px-2.5",
          "text-small text-faint-foreground transition-colors duration-150 hover:border-border-strong hover:text-muted-foreground ",
        )}
      >
        <span className="hidden sm:inline">{t("header.jumpTo")}</span>
        <Command size={17} className="sm:hidden" />
        <kbd className="hidden sm:block rounded border border-border bg-surface-2 px-1 font-mono text-[10px] text-faint-foreground">
          {/Mac|iPhone|iPad/.test(navigator.platform) ? "⌘ K" : "Ctrl K"}
        </kbd>
      </button>
    </header>
  );
}
