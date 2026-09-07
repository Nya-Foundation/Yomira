import { Link } from "react-router-dom";
import type { TagGroup } from "../../lib/types";
import { NAMESPACE_COLORS } from "../../lib/categories";
import { useT } from "../../lib/i18n";

/**
 * Tag list grouped by namespace, one pill per tag. Pills link back to the
 * gallery list filtered by `namespace:tag`; when a translation exists (zh-CN
 * with the tag DB available) the pill shows it and keeps the raw tag as a
 * hover title.
 */
export function TagSection({
  tags,
  translated,
}: {
  tags: TagGroup[];
  translated: (namespace: string, name: string) => string | null;
}) {
  const t = useT();
  return (
    <section aria-label={t("detail.tags")} className="mt-8">
      <h3 className="mb-3 text-emphasis font-semibold tracking-tight">{t("detail.tags")}</h3>
      {tags.length === 0 ? (
        <p className="text-small text-faint-foreground">{t("detail.noTags")}</p>
      ) : (
        <div className="space-y-2.5">
          {tags.map((group) => (
            <div key={group.namespace} className="flex flex-wrap items-center gap-1.5">
              <span className="flex w-24 shrink-0 items-center gap-1.5 text-caption font-medium uppercase tracking-wide text-faint-foreground">
                <span
                  aria-hidden
                  className="h-1.5 w-1.5 shrink-0 rounded-full"
                  style={{ backgroundColor: NAMESPACE_COLORS[group.namespace] ?? NAMESPACE_COLORS.misc }}
                />
                {group.namespace}
              </span>
              {group.tags.map((tag) => {
                const text = translated(group.namespace, tag);
                return (
                  <Link
                    key={tag}
                    to={`/?search=${encodeURIComponent(`${group.namespace}:${tag.replace(/ /g, "_")}`)}`}
                    title={text ? tag : undefined}
                    className="rounded-full border border-border bg-surface px-2.5 py-0.5 text-small text-muted-foreground transition-colors duration-150 hover:border-accent/40 hover:text-accent"
                  >
                    {text ?? tag}
                  </Link>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
