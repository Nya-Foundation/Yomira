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
  return (
    <div className="relative">
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
        <div className="absolute left-0 top-full z-20 mt-1 flex gap-1.5 rounded-lg border border-border bg-elevated p-2 shadow-xl shadow-black/40">
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
