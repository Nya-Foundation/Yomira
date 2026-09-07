import { Heart } from "lucide-react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

export function RatingHearts({
  value,
  interactive = false,
  onRate,
  size = 12,
  className,
}: {
  value: number; // 0..5
  interactive?: boolean;
  onRate?: (rating: number) => void;
  size?: number;
  className?: string;
}) {
  const t = useT();
  return (
    <span className={cn("inline-flex items-center gap-0.5", className)}>
      {[1, 2, 3, 4, 5].map((i) => {
        const filled = value >= i - 0.25;
        const heart = (
          <Heart
            size={size}
            className={
              filled
                ? "fill-accent text-accent"
                : interactive
                  ? ""
                  : "text-faint-foreground"
            }
          />
        );
        return interactive ? (
          <button
            key={i}
            aria-label={i > 1 ? t("rating.stars", { n: i }) : t("rating.star", { n: i })}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onRate?.(i);
            }}
            className="rounded p-0.5 text-faint-foreground transition-colors duration-150 hover:text-accent"
          >
            {heart}
          </button>
        ) : (
          <span key={i}>{heart}</span>
        );
      })}
    </span>
  );
}
