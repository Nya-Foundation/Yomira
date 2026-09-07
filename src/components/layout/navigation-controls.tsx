import { ArrowLeft, ArrowRight } from "lucide-react";
import { useAppNavigation } from "../../lib/app-navigation";
import { useT } from "../../lib/i18n";

export function NavigationControls() {
  const { canGoBack, canGoForward, back, forward } = useAppNavigation();
  const t = useT();
  const style = "flex size-11 min-h-[44px] min-w-[44px] shrink-0 items-center justify-center rounded-lg text-muted-foreground hover:bg-surface-2 hover:text-foreground disabled:pointer-events-none disabled:opacity-30";
  return <div className="flex shrink-0 items-center gap-0.5">
    <button type="button" className={style} disabled={!canGoBack} onClick={back} aria-label={t("nav.back")} title={`${t("nav.back")} (Alt+←)`}><ArrowLeft size={18} /></button>
    <button type="button" className={style} disabled={!canGoForward} onClick={forward} aria-label={t("nav.forward")} title={`${t("nav.forward")} (Alt+→)`}><ArrowRight size={18} /></button>
  </div>;
}
