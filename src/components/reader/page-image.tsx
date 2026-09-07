import { RefreshCw } from "lucide-react";
import { useContext, useState, type ComponentProps } from "react";
import { ApiError } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { Button } from "../ui/button";

import { OfflineContext } from "./page-query";

export function ReaderImage({ onRetry, ...props }: ComponentProps<"img"> & { onRetry: () => void }) {
  const [failed, setFailed] = useState(false);
  const offline = useContext(OfflineContext);
  const t = useT();
  if (failed) {
    return <PageErrorCard
      error={new ApiError(404, t("reader.imageUnreadable"), offline ? "pageUnavailable" : "eh")}
      onRetry={onRetry}
    />;
  }
  return <img {...props} onError={() => setFailed(true)} />;
}

export function PageErrorCard({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  const t = useT();
  const unavailable = error instanceof ApiError && error.kind === "pageUnavailable";
  const is509 =
    error instanceof ApiError &&
    (error.code === 509 || error.kind === "image509" || error.kind === "bandwidthExceeded");
  return (
    <div className="z-10 flex max-w-sm flex-col items-center gap-3 rounded-lg border border-danger/25 bg-danger/5 px-8 py-8 text-center">
      <p className="text-base font-semibold text-danger">
        {t(unavailable ? "reader.pageNotSaved" : is509 ? "reader.509title" : "reader.pageFailed")}
      </p>
      <p className="text-caption leading-5 text-muted-foreground">
        {unavailable ? t("reader.pageNotSavedDesc") : is509
          ? t("reader.509desc")
          : error instanceof Error
            ? error.message
            : t("common.unknownError")}
      </p>
      <Button variant="outline" size="sm" onClick={onRetry} className="gap-1.5">
        <RefreshCw size={13} strokeWidth={1.75} /> {t("common.retry")}
      </Button>
    </div>
  );
}
