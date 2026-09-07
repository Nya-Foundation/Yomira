import { useRef } from "react";
import { getBuiltinHosts } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { toast } from "../../stores/toast";
import { Button } from "../ui/button";
import { Textarea } from "../ui/textarea";

export function HostsEditor({ value, onChange }: { value: string; onChange: (text: string) => void }) {
  const t = useT();
  const file = useRef<HTMLInputElement>(null);
  return (
    <section className="space-y-3 py-4">
      <div><label htmlFor="hosts-editor" className="text-base font-medium">{t("settings.network.hosts")}</label>
        <p className="mt-1 text-small leading-5 text-faint-foreground">{t("settings.network.hostsHint")}</p></div>
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" size="sm" onClick={() => void getBuiltinHosts().then(onChange).catch((e) => toast.error(t("settings.network.applyFailed"), String(e)))}>{t("settings.network.builtinHosts")}</Button>
        <Button variant="outline" size="sm" onClick={() => file.current?.click()}>{t("settings.network.importHosts")}</Button>
        <Button variant="ghost" size="sm" onClick={() => onChange("")}>{t("settings.network.systemDns")}</Button>
        <input ref={file} type="file" accept=".txt,text/plain" className="hidden" aria-label={t("settings.network.importHosts")}
          onChange={async (e) => {
            const selected = e.currentTarget.files?.[0];
            e.currentTarget.value = "";
            if (!selected) return;
            try {
              if (selected.size > 1024 * 1024) throw new Error(t("settings.network.hostsTooLarge"));
              onChange(await selected.text());
            } catch (error) { toast.error(t("settings.network.applyFailed"), String(error)); }
          }} />
      </div>
      <Textarea id="hosts-editor" value={value} onChange={(e) => onChange(e.target.value)}
        spellCheck={false} autoCapitalize="none" autoCorrect="off" className="min-h-48 font-mono text-small"
        placeholder="104.20.18.168 e-hentai.org" />
    </section>
  );
}
