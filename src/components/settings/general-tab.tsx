import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { LogOut } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { checkSignIn, signOut } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { useSettings, type AppSettings } from "../../lib/settings";
import { toast } from "../../stores/toast";
import { Button } from "../ui/button";
import { Select } from "../ui/select";

import { Row } from "./settings-row";
export function GeneralTab() {
  const t = useT();
  const galleryView = useSettings((s) => s.settings.galleryView);
  const density = useSettings((s) => s.settings.density);
  const displayScale = useSettings((s) => s.settings.displayScale);
  const language = useSettings((s) => s.settings.language);
  const update = useSettings((s) => s.update);
  return (
    <div className="divide-y divide-border">
      <Row label={t("settings.general.theme")} hint={t("settings.general.themeHint")}>
        <ThemePicker />
      </Row>
      <Row label={t("list.galleryView")}>
        <Select aria-label={t("list.galleryView")} value={galleryView}
          onChange={(e) => update({ galleryView: e.target.value as AppSettings["galleryView"] })} className="w-40">
          <option value="thumbnail">{t("list.thumbnailView")}</option>
          <option value="detail">{t("list.detailView")}</option>
        </Select>
      </Row>
      <Row label={t("settings.general.density")} hint={t("settings.general.densityHint")}>
        <Select
          aria-label={t("settings.general.density")}
          value={density}
          onChange={(e) => update({ density: e.target.value as AppSettings["density"] })}
          className="w-40"
        >
          <option value="comfortable">{t("settings.general.comfortable")}</option>
          <option value="compact">{t("settings.general.compact")}</option>
        </Select>
      </Row>
      <Row label={t("settings.general.displayScale")} hint={t("settings.general.displayScaleHint")}>
        <Select
          aria-label={t("settings.general.displayScale")}
          value={String(displayScale)}
          onChange={(e) => update({ displayScale: Number(e.target.value) })}
          className="w-40"
        >
          <option value="0.75">75%</option>
          {![0.75, 1, 1.25, 1.5, 1.75, 2].includes(displayScale) && <option value={String(displayScale)}>{Math.round(displayScale * 100)}%</option>}
          <option value="1">100%</option>
          <option value="1.25">125%</option>
          <option value="1.5">150%</option>
          <option value="1.75">175%</option>
          <option value="2">200%</option>
        </Select>
      </Row>
      <Row label={t("settings.general.language")} hint={t("settings.general.languageHint")}>
        <Select
          aria-label={t("settings.general.language")}
          value={language}
          onChange={(e) => update({ language: e.target.value as AppSettings["language"] })}
          className="w-40"
        >
          <option value="system">{t("settings.general.langSystem")}</option>
          <option value="en">{t("settings.general.langEn")}</option>
          <option value="zh-CN">{t("settings.general.langZh")}</option>
        </Select>
      </Row>
      <AccountRow />
    </div>
  );
}

function ThemePicker() {
  const t = useT();
  const theme = useSettings((s) => s.settings.theme);
  const update = useSettings((s) => s.update);
  return (
    <Select
      aria-label={t("settings.general.theme")}
          value={theme}
      onChange={(e) => update({ theme: e.target.value as AppSettings["theme"] })}
      className="w-40"
    >
      <option value="dark">{t("settings.general.dark")}</option>
      <option value="light">{t("settings.general.light")}</option>
    </Select>
  );
}

function AccountRow() {
  const t = useT();
  const { data: signedIn } = useQuery({ queryKey: ["signin"], queryFn: checkSignIn });
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const mutation = useMutation({
    mutationFn: signOut,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["signin"] });
      toast.info(t("settings.general.signedOut"));
      void navigate("/signin");
    },
  });
  return (
    <Row
      label={t("settings.general.account")}
      hint={signedIn ? t("settings.general.accountIn") : t("settings.general.accountOut")}
    >
      {signedIn ? (
        <Button variant="outline" size="sm" onClick={() => mutation.mutate()} className="gap-1.5">
          <LogOut size={13} /> {t("common.signOut")}
        </Button>
      ) : (
        <Button variant="outline" size="sm" onClick={() => navigate("/signin")}>
          {t("common.signIn")}
        </Button>
      )}
    </Row>
  );
}
