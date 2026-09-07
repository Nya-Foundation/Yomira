import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { tagDbStatus, updateTagDb } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { toast } from "../../stores/toast";
import { Button } from "../ui/button";

import { Row } from "./settings-row";
export function SiteTab() {
  const t = useT();
  return (
    <div className="divide-y divide-border">
      <TagDatabaseRow />
      <p className="py-3 text-caption leading-5 text-faint-foreground">
        {t("settings.site.note")}
      </p>
    </div>
  );
}

function TagDatabaseRow() {
  const t = useT();
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["tag-db-status"], queryFn: tagDbStatus });
  const update = useMutation({
    mutationFn: updateTagDb,
    onSuccess: () => {
      toast.success(t("settings.site.updated"));
      void queryClient.invalidateQueries({ queryKey: ["tag-db-status"] });
    },
    onError: (e) =>
      toast.error(t("settings.site.updateFailed"), e instanceof Error ? e.message : undefined),
  });
  const status = query.data;
  return (
    <Row
      label={t("settings.site.tagDb")}
      hint={
        status?.available
          ? t("settings.site.tagDbOk", {
              n: status.entryCount.toLocaleString(),
              v: status.version.slice(0, 7) || t("settings.site.versionUnknown"),
            })
          : t("settings.site.tagDbMissing")
      }
    >
      <Button
        variant="outline"
        size="sm"
        onClick={() => update.mutate()}
        disabled={update.isPending}
      >
        {update.isPending ? t("settings.site.updating") : t("settings.site.update")}
      </Button>
    </Row>
  );
}
