import { formatHosts, parseHosts } from "../../lib/hosts";
import { HostsEditor } from "./hosts-editor";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { getNetworkConfig, setNetworkConfig } from "../../lib/api";
import { useT } from "../../lib/i18n";
import type { NetworkConfig } from "../../lib/types";
import { toast } from "../../stores/toast";
import { Button } from "../ui/button";
import { ErrorState } from "../ui/error-state";
import { Input } from "../ui/input";
import { Select } from "../ui/select";
import { Skeleton } from "../ui/skeleton";
import { Switch } from "../ui/switch";

import { Row } from "./settings-row";
export function NetworkTab() {
  const t = useT();
  const queryClient = useQueryClient();
  const configQuery = useQuery({ queryKey: ["network-config"], queryFn: getNetworkConfig });
  const [hostsText, setHostsText] = useState("");
  const [draft, setDraft] = useState<NetworkConfig | null>(null);

  useEffect(() => {
    if (configQuery.data) { setDraft(configQuery.data); setHostsText(formatHosts(configQuery.data.hosts)); }
  }, [configQuery.data]);

  const apply = useMutation({
    mutationFn: setNetworkConfig,
    onSuccess: (applied, config) => {
      setDraft(config);
      queryClient.setQueryData(["network-config"], config);
      for (const key of ["page", "gallery-list", "gallery-detail"]) {
        void queryClient.invalidateQueries({ queryKey: [key] });
      }
      if (applied) {
        toast.success(t("settings.network.applied"), t("settings.network.appliedDesc"));
      } else {
        toast.info(t("settings.network.savedLocal"), t("settings.network.savedLocalDesc"));
      }
    },
    onError: (e) =>
      toast.error(t("settings.network.applyFailed"), e instanceof Error ? e.message : undefined),
  });

  if (configQuery.isError) {
    return (
      <ErrorState
        description={t("settings.network.loadError")}
        onRetry={() => void configQuery.refetch()}
        className="py-6"
      />
    );
  }

  if (!draft) {
    return (
      <div className="space-y-3 py-3">
        {Array.from({ length: 6 }, (_, i) => (
          <div key={i} className="flex items-center justify-between gap-6 py-1">
            <div className="space-y-1.5">
              <Skeleton className="h-4 w-28 rounded" />
              <Skeleton className="h-3 w-44 rounded" />
            </div>
            <Skeleton className="h-9 w-40 rounded-lg" />
          </div>
        ))}
      </div>
    );
  }

  const patch = (p: Partial<NetworkConfig>) => setDraft((d) => (d ? { ...d, ...p } : d));
  const proxyOff = draft.proxyType === "none";

  return (
    <div className="divide-y divide-border">
      <Row label={t("settings.network.site")} hint={t("settings.network.siteHint")}>
        <Select
          aria-label={t("settings.network.site")}
          value={draft.site}
          onChange={(e) => patch({ site: e.target.value })}
          className="w-40"
        >
          <option value="e">{t("settings.network.ehentai")}</option>
          <option value="ex">{t("settings.network.exhentai")}</option>
        </Select>
      </Row>
      <Row label={t("settings.network.proxyType")}>
        <Select
          aria-label={t("settings.network.proxyType")}
          value={draft.proxyType}
          onChange={(e) => patch({ proxyType: e.target.value })}
          className="w-40"
        >
          <option value="none">{t("settings.network.proxyNone")}</option>
          <option value="http">HTTP</option>
          <option value="socks5">SOCKS5</option>
        </Select>
      </Row>
      <Row label={t("settings.network.proxyHost")}>
        <Input
          value={draft.proxyHost}
          onChange={(e) => patch({ proxyHost: e.target.value })}
          className="w-52 font-mono text-small"
          disabled={proxyOff}
          aria-label={t("settings.network.proxyHost")}
        />
      </Row>
      <Row label={t("settings.network.proxyPort")}>
        <Input
          type="number"
          min={1}
          max={65535}
          value={draft.proxyPort}
          onChange={(e) => patch({ proxyPort: Number(e.target.value) || 0 })}
          className="w-24"
          disabled={proxyOff}
          aria-label={t("settings.network.proxyPort")}
        />
      </Row>
      <Row label={t("settings.network.doh")} hint={t("settings.network.dohHint")}>
        <Switch
          checked={draft.dohEnabled}
          onCheckedChange={(v) => patch({ dohEnabled: v })}
          aria-label={t("settings.network.dohAria")}
        />
      </Row>
      <Row label={t("settings.network.imageRes")} hint={t("settings.network.imageResHint")}>
        <Select aria-label={t("settings.network.imageRes")} value={draft.imageResolution ?? "a"}
          onChange={(e) => patch({ imageResolution: e.target.value })} className="w-40">
          <option value="a">{t("settings.network.imageAuto")}</option>
          {[780, 1280, 1600, 2400].map((width) => <option key={width} value={width}>{width}px</option>)}
        </Select>
      </Row>
      <Row label={t("settings.network.thumbRes")} hint={t("settings.network.thumbHint")}>
        <Select
          aria-label={t("settings.network.thumbRes")}
          value={String(draft.thumbResolution)}
          onChange={(e) => patch({ thumbResolution: Number(e.target.value) })}
          className="w-40"
        >
          <option value={1}>{t("settings.network.thumbLow")}</option>
          <option value={2}>{t("settings.network.thumbMedium")}</option>
          <option value={0}>{t("settings.network.thumbHigh")}</option>
        </Select>
      </Row>
      <Row label={t("settings.network.fixThumb")} hint={t("settings.network.fixThumbHint")}>
        <Switch checked={draft.fixThumbUrl} onCheckedChange={(fixThumbUrl) => patch({ fixThumbUrl })} aria-label={t("settings.network.fixThumb")} />
      </Row>
      <Row label={t("settings.network.cacheSize")} hint={t("settings.network.cacheSizeHint")}>
        <Input type="number" min={64} max={16384} step={64} value={draft.imageCacheSizeMb}
          onChange={(e) => patch({ imageCacheSizeMb: Math.max(64, Math.min(16384, Number(e.target.value) || 64)) })}
          aria-label={t("settings.network.cacheSize")} className="w-28" />
      </Row>
      <HostsEditor value={hostsText} onChange={setHostsText} />
      <div className="flex items-center justify-between py-3.5">
        <p className="text-caption leading-5 text-faint-foreground">
          {t("settings.network.note")}
        </p>
        <Button
          size="sm"
          onClick={() => {
            try { apply.mutate({ ...draft, imageResolution: draft.imageResolution ?? "a", hosts: parseHosts(hostsText) }); }
            catch (error) { toast.error(t("settings.network.applyFailed"), error instanceof Error ? error.message : String(error)); }
          }}
          disabled={apply.isPending}
        >
          {apply.isPending ? t("settings.network.applying") : t("settings.network.apply")}
        </Button>
      </div>
    </div>
  );
}
