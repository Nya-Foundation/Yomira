import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { User } from "lucide-react";
import { checkSignIn } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { Tooltip } from "../ui/tooltip";

export function UserButton() {
  const t = useT();
  const { data: signedIn } = useQuery({
    queryKey: ["signin"],
    queryFn: checkSignIn,
  });

  return (
    <Tooltip content={signedIn ? t("account.signedIn") : t("account.signIn")} side="right">
      <Link
        to="/signin"
        aria-label={t("account.aria")}
        className="flex h-9 w-9 items-center justify-center rounded-full border border-border bg-surface-2 text-muted-foreground transition-colors duration-150 hover:text-foreground"
      >
        {signedIn ? (
          <span className="flex h-6 w-6 items-center justify-center rounded-full bg-accent/20 text-caption font-semibold text-accent">
            U
          </span>
        ) : (
          <User size={16} strokeWidth={1.75} />
        )}
      </Link>
    </Tooltip>
  );
}
