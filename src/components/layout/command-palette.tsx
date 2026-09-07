import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { CornerDownLeft, Search, Clock } from "lucide-react";
import { Dialog } from "../ui/dialog";
import { Input } from "../ui/input";
import { getHistory } from "../../lib/api";
import { ROUTE_META } from "./header";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

interface Item {
  key: string;
  label: string;
  hint: string;
  isRecent: boolean;
  run: () => void;
}

export function CommandPalette({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const navigate = useNavigate();
  const t = useT();
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setIndex(0);
    }
  }, [open]);

  const historyQuery = useQuery({
    queryKey: ["history-recent"],
    queryFn: () => getHistory(0),
    staleTime: 30_000,
  });

  const items = useMemo<Item[]>(() => {
    const q = query.trim().toLowerCase();
    const routes = ROUTE_META.filter((r) => r.path.split("/").length <= 2).map((r) => ({
      key: `route:${r.path}`,
      label: t(r.labelKey),
      hint: r.path,
      isRecent: false,
      run: () => navigate(r.path),
    }));
    const recents = (historyQuery.data ?? []).slice(0, 8).map((h) => ({
      key: `gallery:${h.gid}`,
      label: h.title,
      hint: t("palette.recent"),
      isRecent: true,
      run: () => navigate(`/gallery/${h.gid}/${h.token}`),
    }));
    const all = [...routes, ...recents];
    if (!q) return all;
    return all.filter((i) => i.label.toLowerCase().includes(q) || i.hint.toLowerCase().includes(q));
  }, [query, navigate, historyQuery.data, t]);

  useEffect(() => setIndex(0), [items.length]);

  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${index}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [index]);

  const select = (item: Item) => {
    onOpenChange(false);
    item.run();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="" className="max-w-lg">
      <div className="-m-4">
        <div className="flex items-center gap-2 border-b border-border px-3">
          <Search size={15} strokeWidth={1.75} className="shrink-0 text-faint-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setIndex((i) => Math.min(i + 1, items.length - 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setIndex((i) => Math.max(i - 1, 0));
              } else if (e.key === "Enter" && items[index]) {
                e.preventDefault();
                select(items[index]);
              }
            }}
            placeholder={t("palette.placeholder")}
            className="h-11 rounded-none border-0 bg-transparent px-0 hover:border-0 focus:border-0"
            autoFocus
          />
        </div>
        <div ref={listRef} className="max-h-80 overflow-y-auto p-1.5">
          {items.length === 0 && (
            <p className="px-3 py-6 text-center text-base text-muted-foreground">{t("palette.noResults")}</p>
          )}
          {items.map((item, i) => (
            <button
              key={item.key}
              data-index={i}
              onMouseEnter={() => setIndex(i)}
              onClick={() => select(item)}
              className={cn(
                "flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-base",
                i === index ? "bg-surface-2 text-foreground" : "text-muted-foreground",
              )}
            >
              {item.isRecent ? (
                <Clock size={14} strokeWidth={1.75} className="shrink-0 text-faint-foreground" />
              ) : (
                <CornerDownLeft size={14} strokeWidth={1.75} className="shrink-0 text-faint-foreground" />
              )}
              <span className="flex-1 truncate">{item.label}</span>
              <span className="shrink-0 text-caption text-faint-foreground">{item.hint}</span>
            </button>
          ))}
        </div>
      </div>
    </Dialog>
  );
}
