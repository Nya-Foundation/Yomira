import { useQuery } from "@tanstack/react-query";
import { RotateCw } from "lucide-react";
import { getImage } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

/**
 * Image that resolves its `src` through the backend image cache. In the app,
 * `getImage` returns a loopback HTTP URL served from the Rust-side
 * cache; in the browser demo the (data-URL) mock passes through untouched.
 * The resolved URL mapping is cached app-wide by react-query, so galleries
 * rendering the same thumb more than once resolve only once.
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
  const query = useQuery({
    queryKey: ["image-url", url],
    queryFn: () => getImage(url),
    staleTime: Infinity,
    retry: false,
  });

  if (query.isPending) {
    return <div role="img" aria-label={alt} className={cn("animate-pulse bg-surface-2", className)} style={style} />;
  }

  if (query.isError) {
    return (
      <div
        role="img"
        aria-label={alt}
        className={cn("flex items-center justify-center bg-surface-2", className)}
        style={style}
      >
        <button
          onClick={() => void query.refetch()}
          aria-label={t("image.retryAria", { name: alt || t("image.fallback") })}
          title={query.error instanceof Error ? query.error.message : t("image.retryTitle")}
          className="rounded-full p-1.5 text-faint-foreground transition-colors hover:bg-surface hover:text-foreground"
        >
          <RotateCw size={15} strokeWidth={1.75} />
        </button>
      </div>
    );
  }

  return (
    <img
      src={query.data}
      alt={alt}
      loading={loading}
      decoding="async"
      draggable={false}
      referrerPolicy={referrerPolicy}
      className={className}
      style={style}
    />
  );
}
