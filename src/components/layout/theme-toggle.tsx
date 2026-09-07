import { Moon, Sun } from "lucide-react";
import { useSettings } from "../../lib/settings";
import { useT } from "../../lib/i18n";
import { Tooltip } from "../ui/tooltip";

export function ThemeToggle() {
  const theme = useSettings((s) => s.settings.theme);
  const update = useSettings((s) => s.update);
  const t = useT();
  const isDark = theme !== "light";
  return (
    <Tooltip content={isDark ? t("theme.toLight") : t("theme.toDark")} side="right">
      <button
        onClick={() => update({ theme: isDark ? "light" : "dark" })}
        aria-label={t("theme.toggle")}
        className="flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors duration-150 hover:bg-surface-2 hover:text-foreground"
      >
        {isDark ? <Sun size={18} strokeWidth={1.75} /> : <Moon size={18} strokeWidth={1.75} />}
      </button>
    </Tooltip>
  );
}
