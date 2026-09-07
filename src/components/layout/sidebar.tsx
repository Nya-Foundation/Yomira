import { useState } from "react";
import { NavLink, useLocation } from "react-router-dom";
import {
  Home,
  Flame,
  Eye,
  Trophy,
  Heart,
  Download,
  History,
  Newspaper,
  Settings,
  Menu,
} from "lucide-react";
import { cn } from "../../lib/utils";
import { useT, type TKey } from "../../lib/i18n";
import { Sheet } from "../ui/sheet";
import { ThemeToggle } from "./theme-toggle";
import { UserButton } from "./user-button";

const NAV: { to: string; labelKey: TKey; icon: typeof Home; end?: boolean }[] = [
  { to: "/", labelKey: "nav.home", icon: Home, end: true },
  { to: "/popular", labelKey: "nav.popular", icon: Flame },
  { to: "/watched", labelKey: "nav.watched", icon: Eye },
  { to: "/toplist", labelKey: "nav.toplist", icon: Trophy },
  { to: "/favorites", labelKey: "nav.favorites", icon: Heart },
  { to: "/downloads", labelKey: "nav.downloads", icon: Download },
  { to: "/history", labelKey: "nav.history", icon: History },
  { to: "/news", labelKey: "nav.news", icon: Newspaper },
];

export function Sidebar() {
  const t = useT();
  const { pathname } = useLocation();
  const [moreOpen, setMoreOpen] = useState(false);
  const primary = NAV.filter((item) => ["/", "/favorites", "/downloads"].includes(item.to));
  const secondary = [...NAV.filter((item) => !primary.includes(item)), { to: "/settings", labelKey: "nav.settings" as TKey, icon: Settings }];
  const navLink = "flex min-h-11 items-center gap-3 rounded-lg px-3 text-small font-medium transition-colors";

  return (
    <>
      <nav aria-label="Primary" className="hidden w-16 shrink-0 flex-col border-r border-border bg-elevated md:flex xl:w-48">
        <div className="flex h-16 shrink-0 items-center justify-center px-3 xl:justify-start">
          <img src="/yomira.svg" alt="Yomira" className="size-9 shrink-0" />
          <span aria-hidden="true" className="ml-3 hidden font-semibold tracking-tight xl:block">Yomira</span>
        </div>
        <div className="min-h-0 flex-1 space-y-1 overflow-y-auto px-2 py-3">
          {NAV.map(({ to, labelKey, icon: Icon, end }, index) => (
            <NavLink key={to} to={to} end={end} aria-label={t(labelKey)} title={t(labelKey)}
              className={({ isActive }) => cn(navLink, "justify-center xl:justify-start", index === 4 && "mt-5",
                isActive ? "bg-accent/10 text-accent" : "text-muted-foreground hover:bg-surface-2 hover:text-foreground")}>
              <Icon size={19} strokeWidth={1.75} className="shrink-0" />
              <span className="hidden xl:block">{t(labelKey)}</span>
            </NavLink>
          ))}
        </div>
        <div className="space-y-2 border-t border-border px-2 py-3">
          <NavLink to="/settings" aria-label={t("nav.settings")} title={t("nav.settings")}
            className={({ isActive }) => cn(navLink, "justify-center xl:justify-start", isActive ? "bg-accent/10 text-accent" : "text-muted-foreground hover:bg-surface-2 hover:text-foreground")}>
            <Settings size={19} strokeWidth={1.75} className="shrink-0" /><span className="hidden xl:block">{t("nav.settings")}</span>
          </NavLink>
          <div className="flex flex-col items-center gap-2 xl:flex-row xl:justify-between xl:px-2"><UserButton /><ThemeToggle /></div>
        </div>
      </nav>

      <nav aria-label="Primary" className="mobile-nav fixed inset-x-0 bottom-0 z-40 grid grid-cols-4 border-t border-border bg-elevated md:hidden">
        {primary.map(({ to, labelKey, icon: Icon, end }) => (
          <NavLink key={to} to={to} end={end} aria-label={t(labelKey)}
            className={({ isActive }) => cn("flex min-h-16 min-w-0 flex-col items-center justify-center gap-1 text-caption transition-colors", isActive ? "text-accent" : "text-muted-foreground")}>
            <Icon size={20} strokeWidth={1.75} /><span className="max-w-full truncate px-1">{t(labelKey)}</span>
          </NavLink>
        ))}
        <button onClick={() => setMoreOpen(true)} aria-haspopup="dialog" aria-expanded={moreOpen}
          className={cn("flex min-h-16 flex-col items-center justify-center gap-1 text-caption hover:text-foreground", secondary.some((item) => item.to === pathname) ? "text-accent" : "text-muted-foreground")}>
          <Menu size={20} strokeWidth={1.75} /><span className="max-w-full truncate px-1">{t("nav.more")}</span>
        </button>
      </nav>
      <Sheet open={moreOpen} onOpenChange={setMoreOpen} title={t("nav.more")}>
        <nav aria-label={t("nav.more")} className="space-y-1">
          {secondary.map(({ to, labelKey, icon: Icon }) => (
            <NavLink key={to} to={to} onClick={() => setMoreOpen(false)}
              className={({ isActive }) => cn(navLink, isActive ? "bg-accent/10 text-accent" : "text-muted-foreground hover:bg-surface-2")}>
              <Icon size={20} strokeWidth={1.75} /><span className="max-w-full truncate px-1">{t(labelKey)}</span>
            </NavLink>
          ))}
        </nav>
        <div className="mt-5 flex items-center justify-between border-t border-border pt-4"><UserButton /><ThemeToggle /></div>
      </Sheet>
    </>
  );
}
