import { useState } from "react";
import { RotateCw } from "lucide-react";
import { imageSrc } from "../../lib/image-source";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

/**
 * Image served through the backend's loopback image server. The `src` is
 * derived from the remote URL with no IPC, so the browser keeps its own
 * lazy-loading and request priorities and the backend fetches on first
 * request. Failures offer a retry, which remounts the element; error
 * responses are sent `no-store`, so the request actually repeats.
 */
export function CachedImage({
  url,
  alt = "",
  className,
  style,
  loading = "lazy",
  referrerPolicy,
}: {
  url: string;
  alt?: string;
  className?: string;
  style?: React.CSSProperties;
  loading?: "lazy" | "eager";
  referrerPolicy?: React.ImgHTMLAttributes<HTMLImageElement>["referrerPolicy"];
}) {
  const t = useT();
  const [attempt, setAttempt] = useState(0);
  const [failed, setFailed] = useState<string | null>(null);
  const [shown, setShown] = useState<string | null>(null);

  const src = imageSrc(url);
  const key = `${src ?? ""}#${attempt}`;

  if (src === null || failed === key) {
    return (
      <div
        role="img"
        aria-label={alt}
        className={cn("flex items-center justify-center bg-surface-2", className)}
        style={style}
      >
        <button
          onClick={() => {
            setFailed(null);
            setAttempt((n) => n + 1);
          }}
          aria-label={t("image.retryAria", { name: alt || t("image.fallback") })}
          title={t("image.retryTitle")}
          className="rounded-full p-1.5 text-faint-foreground transition-colors hover:bg-surface hover:text-foreground"
        >
          <RotateCw size={15} strokeWidth={1.75} />
        </button>
      </div>
    );
  }

  return (
    <img
      key={key}
      src={src}
      alt={alt}
      loading={loading}
      decoding="async"
      draggable={false}
      referrerPolicy={referrerPolicy}
      onLoad={() => setShown(key)}
      onError={() => setFailed(key)}
      className={cn(shown === key ? undefined : "animate-pulse bg-surface-2", className)}
      style={style}
    />
  );
}
