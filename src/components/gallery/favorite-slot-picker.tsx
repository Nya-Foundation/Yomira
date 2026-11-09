import { useEffect, useRef } from "react";
import { Heart } from "lucide-react";
import { FAVORITE_SLOTS } from "../../lib/categories";
import { useT } from "../../lib/i18n";
import { Button } from "../ui/button";

/**
 * Favorite button + slot color-dot popover. Picking a dot favors the gallery
 * into that slot; clicking an already-favorited gallery unfavorites it.
 */
export function FavoriteSlotPicker({
  isFavorited,
  favoriteSlot,
  open,
  onOpenChange,
  onPick,
}: {
  isFavorited: boolean;
  favoriteSlot: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onPick: (slot: number) => void;
}) {
  const t = useT();
  const wrapRef = useRef<HTMLDivElement>(null);

  // This popover is not a Modal, so nothing else dismisses it: Escape and the
  // Android back handler both look for an open <dialog>. Without this it could
  // only be closed by favouriting something, and hardware back left the route.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onOpenChange(false);
      }
    };
    const onDown = (e: PointerEvent) => {
      if (!wrapRef.current?.contains(e.target as Node)) onOpenChange(false);
    };
    document.addEventListener("keydown", onKey, true);
    document.addEventListener("pointerdown", onDown, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      document.removeEventListener("pointerdown", onDown, true);
    };
  }, [open, onOpenChange]);

  return (
    <div ref={wrapRef} className="relative">
      <Button
        variant={isFavorited ? "default" : "subtle"}
        onClick={() =>
          isFavorited ? onPick(favoriteSlot) : onOpenChange(!open)
        }
        className="gap-1.5"
      >
        <Heart size={15} strokeWidth={1.75} className={isFavorited ? "fill-accent" : ""} />
        {isFavorited ? t("detail.favoritedSlot", { n: favoriteSlot + 1 }) : t("detail.favorite")}
      </Button>
      {open && !isFavorited && (
        // Ten swatches in a row overflow the viewport once the coarse-pointer
        // floor widens each to 44px, putting the later slots off-screen and
        // giving the page horizontal scroll. Wrap them instead.
        <div className="absolute right-0 top-full z-20 mt-1 grid grid-cols-5 gap-1.5 rounded-lg border border-border bg-elevated p-2 shadow-xl shadow-black/40">
          {Array.from({ length: FAVORITE_SLOTS }, (_, i) => (
            <button
              key={i}
              aria-label={t("detail.favoriteSlot", { n: i + 1 })}
              onClick={() => onPick(i)}
              className="h-6 w-6 rounded-md transition-transform duration-150 hover:scale-110"
              style={{ backgroundColor: i === 0 ? "#fb7185" : `hsl(${i * 36} 60% 55%)` }}
            />
          ))}
        </div>
      )}
    </div>
  );
}
