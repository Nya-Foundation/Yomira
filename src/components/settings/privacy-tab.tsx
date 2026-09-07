import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { X } from "lucide-react";
import { useState } from "react";
import { blacklistCreate, blacklistDelete, blacklistList } from "../../lib/api";
import { useT, type TKey } from "../../lib/i18n";
import type { BlacklistEntry } from "../../lib/types";
import { toast } from "../../stores/toast";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Select } from "../ui/select";
import { Skeleton } from "../ui/skeleton";

const BLACKLIST_TYPES: { value: string; labelKey: TKey }[] = [
  { value: "title", labelKey: "settings.privacy.typeTitle" },
  { value: "uploader", labelKey: "settings.privacy.typeUploader" },
  { value: "tag", labelKey: "settings.privacy.typeTag" },
];

function BlockedContent() {
  const t = useT();
  const queryClient = useQueryClient();
  const [type, setType] = useState("title");
  const [content, setContent] = useState("");

  const query = useQuery({ queryKey: ["blacklist"], queryFn: blacklistList });
  const invalidate = () => void queryClient.invalidateQueries({ queryKey: ["blacklist"] });

  const add = useMutation({
    mutationFn: () => blacklistCreate(type, content.trim()),
    onSuccess: () => {
      setContent("");
      invalidate();
    },
    onError: (e) =>
      toast.error(t("settings.network.addFailed"), e instanceof Error ? e.message : undefined),
  });
  const remove = useMutation({
    mutationFn: (id: number) => blacklistDelete(id),
    onSuccess: invalidate,
  });

  const entries = query.data ?? [];

  return (
    <section className="py-3" aria-label={t("settings.privacy.blocked")}>
      <div className="mb-1 flex items-center justify-between">
        <p className="text-base font-medium">{t("settings.privacy.blocked")}</p>
        <span className="font-mono text-caption text-faint-foreground tabular-nums">
          {entries.length === 1
            ? t("settings.privacy.blockedOne", { n: entries.length })
            : t("settings.privacy.blockedMany", { n: entries.length })}
        </span>
      </div>
      <p className="mb-3 text-small leading-5 text-faint-foreground">
        {t("settings.privacy.blockedDesc")}
      </p>

      <div className="mb-3 flex items-center gap-2">
        <Select value={type} onChange={(e) => setType(e.target.value)} className="w-28" aria-label={t("settings.privacy.blockType")}>
          {BLACKLIST_TYPES.map((tb) => (
            <option key={tb.value} value={tb.value}>
              {t(tb.labelKey)}
            </option>
          ))}
        </Select>
        <Input
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && content.trim()) add.mutate();
          }}
          placeholder={type === "tag" ? t("settings.privacy.placeholderTag") : t("settings.privacy.placeholder")}
          className="flex-1 font-mono text-small"
          aria-label={t("settings.privacy.blockedText")}
        />
        <Button size="sm" onClick={() => add.mutate()} disabled={!content.trim() || add.isPending}>
          {t("common.add")}
        </Button>
      </div>

      {query.isLoading ? (
        <ul className="space-y-2">
          {Array.from({ length: 3 }, (_, i) => (
            <li key={i} className="flex items-center gap-3">
              <Skeleton className="h-3.5 w-16 rounded" />
              <Skeleton className="h-3.5 flex-1 rounded" />
            </li>
          ))}
        </ul>
      ) : entries.length === 0 ? (
        <p className="py-2 text-small text-faint-foreground">{t("settings.privacy.nothing")}</p>
      ) : (
        <ul className="divide-y divide-border rounded-lg border border-border bg-surface">
          {entries.map((b: BlacklistEntry) => (
            <li key={b.id} className="group flex items-center gap-3 px-3 py-2">
              <span className="w-20 shrink-0 text-caption font-medium uppercase tracking-wide text-faint-foreground">
                {b.type}
              </span>
              <span className="min-w-0 flex-1 truncate font-mono text-small text-muted-foreground">
                {b.content}
              </span>
              <button
                aria-label={t("settings.privacy.remove", { content: b.content })}
                onClick={() => remove.mutate(b.id)}
                className="rounded-md p-1 text-faint-foreground opacity-0 transition-all duration-150 hover:bg-danger/10 hover:text-danger focus:opacity-100 group-hover:opacity-100"
              >
                <X size={13} strokeWidth={1.75} />
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export function PrivacyTab() {
  return (
    <div className="divide-y divide-border">
      <BlockedContent />
    </div>
  );
}
