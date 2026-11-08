import { useEffect, useState } from "react";
import { categoryLabel, useT, type TKey } from "../../lib/i18n";
import {
  CATEGORIES,
  type Category,
  type ListParams
} from "../../lib/types";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Select } from "../ui/select";
import { Separator } from "../ui/separator";
import { Sheet } from "../ui/sheet";
import { Switch } from "../ui/switch";

const ADV_SEARCH_TOGGLES: Array<{
  field: "searchDesc" | "searchTorrent" | "searchLowPowerTags" | "searchDownvotedTags" | "searchExpunged";
  label: TKey;
  hint: TKey;
}> = [
  { field: "searchDesc", label: "list.sheet.searchDesc", hint: "list.sheet.searchDescHint" },
  { field: "searchTorrent", label: "list.sheet.searchTorrent", hint: "list.sheet.searchTorrentHint" },
  { field: "searchLowPowerTags", label: "list.sheet.searchLowPowerTags", hint: "list.sheet.searchLowPowerTagsHint" },
  { field: "searchDownvotedTags", label: "list.sheet.searchDownvotedTags", hint: "list.sheet.searchDownvotedTagsHint" },
  { field: "searchExpunged", label: "list.sheet.searchExpunged", hint: "list.sheet.searchExpungedHint" },
];

interface FilterSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  params: ListParams;
  setParams: React.Dispatch<React.SetStateAction<ListParams>>;
}


/**
 * Sheet edits are held locally and committed once, on exit.
 *
 * Writing straight through meant every chip and switch changed the list query
 * key, so configuring four categories and a few switches queued that many
 * listing requests at 500ms apart — for states the user was still composing,
 * ahead of the one they wanted. It also left "Apply" doing nothing but close.
 */
function useDraftParams(open: boolean, params: ListParams) {
  const [draft, setDraft] = useState(params);
  // Seeded when the sheet opens, not whenever params change, so editing a
  // value cannot be undone by the commit it causes.
  useEffect(() => {
    if (open) setDraft(params);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
  return [draft, setDraft] as const;
}

export function CategorySheet({ open, onOpenChange, params, setParams }: FilterSheetProps) {
  const [draft, setDraft] = useDraftParams(open, params);
  const close = (next: boolean) => {
    if (!next) setParams(draft);
    onOpenChange(next);
  };
  const t = useT();
  return (
      <Sheet open={open} onOpenChange={close} title={t("list.sheet.categoriesTitle")}>
        <p className="mb-3 text-small leading-5 text-muted-foreground">
          {t("list.sheet.categoriesDesc")}
        </p>
        <div className="flex flex-wrap gap-1.5">
          {CATEGORIES.map((c: Category) => {
            const selected = draft.categories?.includes(c) ?? false;
            return (
              <button
                key={c}
                onClick={() =>
                  setDraft((p) => {
                    const cur = p.categories ?? [];
                    return {
                      ...p,
                      categories: selected
                        ? cur.filter((x) => x !== c)
                        : cur.length === CATEGORIES.length - 1
                          ? null
                          : [...cur, c],
                    };
                  })
                }
                aria-pressed={selected}
                className={cn(
                  "rounded-full border px-2.5 py-1 text-small transition-colors duration-150",
                  selected
                    ? "border-accent/40 bg-accent/15 text-accent"
                    : "border-border bg-surface text-muted-foreground hover:text-foreground",
                )}
              >
                {categoryLabel(c)}
              </button>
            );
          })}
        </div>
        <div className="mt-4">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDraft((p) => ({ ...p, categories: null }))}
          >
            {t("list.sheet.allCategories")}
          </Button>
        </div>
      </Sheet>
  );
}

export function AdvancedSearchSheet({ open, onOpenChange, params, setParams }: FilterSheetProps) {
  const [draft, setDraft] = useDraftParams(open, params);
  const close = (next: boolean) => {
    if (!next) setParams(draft);
    onOpenChange(next);
  };
  const t = useT();
  return (
      <Sheet open={open} onOpenChange={close} title={t("list.sheet.advancedTitle")}>
        <div className="space-y-5">
          <div>
            <label className="mb-1.5 block text-small font-medium text-muted-foreground">
              {t("list.sheet.minRating")}
            </label>
            <Select
              aria-label={t("list.sheet.minRating")}
              value={draft.minRating?.toString() ?? ""}
              onChange={(e) =>
                setDraft((p) => ({
                  ...p,
                  minRating: e.target.value === "" ? null : Number(e.target.value),
                }))
              }
            >
              <option value="">{t("list.sheet.any")}</option>
              {[2, 2.5, 3, 3.5, 4, 4.5, 5].map((r) => (
                <option key={r} value={r}>
                  {t("list.sheet.stars", { n: r })}
                </option>
              ))}
            </Select>
          </div>

          <div>
            <label className="mb-1.5 block text-small font-medium text-muted-foreground">
              {t("list.sheet.pageRange")}
            </label>
            <div className="flex items-center gap-2">
              <Input
                type="number"
                min={0}
                placeholder={t("list.sheet.minPlaceholder")}
                value={draft.minPages ?? ""}
                onChange={(e) =>
                  setDraft((p) => ({
                    ...p,
                    minPages: e.target.value === "" ? null : Number(e.target.value),
                  }))
                }
                aria-label={t("list.sheet.minPages")}
              />
              <span className="text-caption text-faint-foreground">{t("list.sheet.to")}</span>
              <Input
                type="number"
                min={0}
                placeholder={t("list.sheet.maxPlaceholder")}
                value={draft.maxPages ?? ""}
                onChange={(e) =>
                  setDraft((p) => ({
                    ...p,
                    maxPages: e.target.value === "" ? null : Number(e.target.value),
                  }))
                }
                aria-label={t("list.sheet.maxPages")}
              />
            </div>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <p className="text-base">{t("list.sheet.uploaderSearch")}</p>
              <p className="text-caption text-faint-foreground">{t("list.sheet.uploaderSearchHint")}</p>
            </div>
            <Switch
              checked={draft.enableUploader}
              onCheckedChange={(v) => setDraft((p) => ({ ...p, enableUploader: v }))}
              aria-label={t("list.sheet.enableUploader")}
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <p className="text-base">{t("list.sheet.tagSearch")}</p>
              <p className="text-caption text-faint-foreground">{t("list.sheet.tagSearchHint")}</p>
            </div>
            <Switch
              checked={draft.enableTags}
              onCheckedChange={(v) => setDraft((p) => ({ ...p, enableTags: v }))}
              aria-label={t("list.sheet.enableTags")}
            />
          </div>

          {ADV_SEARCH_TOGGLES.map((row) => (
            <div key={row.field} className="flex items-center justify-between">
              <div>
                <p className="text-base">{t(row.label)}</p>
                <p className="text-caption text-faint-foreground">{t(row.hint)}</p>
              </div>
              <Switch
                checked={draft[row.field]}
                onCheckedChange={(v) => setDraft((p) => ({ ...p, [row.field]: v }))}
                aria-label={t(row.label)}
              />
            </div>
          ))}

          <Separator />

          <div className="flex justify-between">
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                setDraft((p) => ({
                  ...p,
                  minRating: null,
                  minPages: null,
                  maxPages: null,
                  enableUploader: true,
                  enableTags: true,
                  searchDesc: false,
                  searchTorrent: false,
                  searchLowPowerTags: false,
                  searchDownvotedTags: false,
                  searchExpunged: false,
                }))
              }
            >
              {t("list.sheet.reset")}
            </Button>
            <Button size="sm" onClick={() => close(false)}>
              {t("list.sheet.apply")}
            </Button>
          </div>
        </div>
      </Sheet>
  );
}
