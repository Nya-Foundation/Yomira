import { useState } from "react";
import { GeneralTab } from "../components/settings/general-tab";
import { NetworkTab } from "../components/settings/network-tab";
import { PrivacyTab } from "../components/settings/privacy-tab";
import { ReaderTab } from "../components/settings/reader-tab";
import { SiteTab } from "../components/settings/site-tab";
import { Tabs } from "../components/ui/tabs";
import { useT, type TKey } from "../lib/i18n";

const TABS: { value: string; labelKey: TKey }[] = [
  { value: "general", labelKey: "settings.tabGeneral" },
  { value: "reader", labelKey: "settings.tabReader" },
  { value: "site", labelKey: "settings.tabSite" },
  { value: "network", labelKey: "settings.tabNetwork" },
  { value: "privacy", labelKey: "settings.tabPrivacy" },
];

export function SettingsPage() {
  const t = useT();
  const [tab, setTab] = useState("general");
  return (
    <div className="h-full overflow-y-auto px-4 py-5 sm:px-6">
      <div className="mx-auto max-w-3xl">
      <Tabs
        value={tab}
        onValueChange={setTab}
        tabs={TABS.map((tb) => ({ value: tb.value, label: t(tb.labelKey) }))}
        className="mb-6"
      />
      {tab === "general" && <GeneralTab />}
      {tab === "reader" && <ReaderTab />}
      {tab === "site" && <SiteTab />}
      {tab === "network" && <NetworkTab />}
      {tab === "privacy" && <PrivacyTab />}
      </div>
    </div>
  );
}
