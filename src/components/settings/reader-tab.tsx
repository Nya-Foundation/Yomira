import { useT } from "../../lib/i18n";
import { useSettings, type AppSettings } from "../../lib/settings";
import { Input } from "../ui/input";
import { Select } from "../ui/select";

import { Row } from "./settings-row";
export function ReaderTab() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  return (
    <div className="divide-y divide-border">
      <Row label={t("settings.reader.mode")} hint={t("settings.reader.modeHint")}>
        <Select
          aria-label={t("settings.reader.mode")}
          value={settings.readerMode}
          onChange={(e) => update({ readerMode: e.target.value as AppSettings["readerMode"] })}
          className="w-40"
        >
          <option value="paged">{t("settings.reader.paged")}</option>
          <option value="continuous">{t("settings.reader.continuous")}</option>
        </Select>
      </Row>
      <Row label={t("settings.reader.direction")} hint={t("settings.reader.directionHint")}>
        <Select aria-label={t("settings.reader.direction")} value={settings.readerDirection}
          onChange={(e) => update({ readerDirection: e.target.value as AppSettings["readerDirection"] })} className="w-40">
          <option value="ltr">{t("settings.reader.ltr")}</option>
          <option value="rtl">{t("settings.reader.rtl")}</option>
        </Select>
      </Row>
      <Row label={t("settings.reader.preload")} hint={t("settings.reader.preloadHint")}>
        <Input
          type="number"
          min={0}
          max={10}
          value={settings.readerPreloadCount}
          onChange={(e) => update({ readerPreloadCount: Math.max(0, Math.min(10, Number(e.target.value) || 0)) })}
          className="w-24"
          aria-label={t("settings.reader.preload")}
        />
      </Row>
    </div>
  );
}
