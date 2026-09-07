import { useEffect, useRef, useState } from "react";
import { Star } from "lucide-react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { Dialog } from "../ui/dialog";
import { Spinner } from "../ui/spinner";

/**
 * 1-10 star rating dialog (EhViewer style). Stars fill on hover, the whole
 * row is keyboard-navigable (arrow keys move the selection, Home/End jump),
 * and confirming reports the selected star count.
 */
export function RatingDialog({
  open,
  onOpenChange,
  initialStars = 0,
  pending = false,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialStars?: number;
  pending?: boolean;
  onSubmit: (stars: number) => void;
}) {
  const t = useT();
  const [selected, setSelected] = useState(initialStars);
  const [hover, setHover] = useState<number | null>(null);
  const starRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const wasOpen = useRef(false);

  // Reset the draft each time the dialog opens with the freshest rating.
  useEffect(() => {
    if (open && !wasOpen.current) {
      setSelected(initialStars);
      setHover(null);
    }
    wasOpen.current = open;
  }, [open, initialStars]);

  const effective = hover ?? selected;

  const move = (to: number) => {
    const next = Math.max(1, Math.min(10, to));
    setSelected(next);
    starRefs.current[next - 1]?.focus();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("rating.title")}>
      <div
        className="flex items-center justify-center gap-1 py-2"
        onMouseLeave={() => setHover(null)}
      >
        {Array.from({ length: 10 }, (_, i) => i + 1).map((i) => (
          <button
            key={i}
            ref={(el) => {
              starRefs.current[i - 1] = el;
            }}
            type="button"
            aria-label={i > 1 ? t("rating.stars", { n: i }) : t("rating.star", { n: i })}
            aria-pressed={selected === i}
            onClick={() => setSelected(i)}
            onFocus={() => setSelected(i)}
            onMouseEnter={() => setHover(i)}
            onKeyDown={(e) => {
              switch (e.key) {
                case "ArrowRight":
                case "ArrowUp":
                  e.preventDefault();
                  move(i + 1);
                  break;
                case "ArrowLeft":
                case "ArrowDown":
                  e.preventDefault();
                  move(i - 1);
                  break;
                case "Home":
                  e.preventDefault();
                  move(1);
                  break;
                case "End":
                  e.preventDefault();
                  move(10);
                  break;
              }
            }}
            className="rounded p-0.5 text-faint-foreground transition-colors duration-100 hover:text-accent focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
          >
            <Star
              size={20}
              className={cn(
                "transition-colors duration-100",
                effective >= i ? "fill-accent text-accent" : "",
              )}
            />
          </button>
        ))}
      </div>
      <p className="mb-4 text-center text-caption text-faint-foreground tabular-nums">
        {t("rating.yourRating", { n: selected })}
      </p>
      <div className="flex justify-end gap-2">
        <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
          {t("common.cancel")}
        </Button>
        <Button
          size="sm"
          disabled={pending || selected === 0}
          onClick={() => onSubmit(selected)}
        >
          {pending && <Spinner size={13} />}
          {t("rating.submit")}
        </Button>
      </div>
    </Dialog>
  );
}
