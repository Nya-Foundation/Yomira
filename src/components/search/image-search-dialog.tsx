import { useRef, useState } from "react";
import { ImagePlus } from "lucide-react";
import { imageLookup } from "../../lib/api";
import type { GalleryListResponse } from "../../lib/types";
import { useT } from "../../lib/i18n";
import { Dialog } from "../ui/dialog";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";

/** Binary -> base64 without blowing the argument stack on big images. */
async function fileToBase64(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let bin = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(bin);
}

export interface ImageSearchResult {
  response: GalleryListResponse;
  filename: string;
}

/**
 * Reverse-image-search dialog: pick a local file, toggle the lookup options
 * and hand the one-shot result set back to the list page for display.
 */
export function ImageSearchDialog({
  open,
  onOpenChange,
  onResults,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onResults: (result: ImageSearchResult) => void;
}) {
  const t = useT();
  const fileRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [similar, setSimilar] = useState(false);
  const [coversOnly, setCoversOnly] = useState(false);
  const [expunged, setExpunged] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState(false);

  const reset = () => {
    setFile(null);
    setError(false);
    setPending(false);
    if (fileRef.current) fileRef.current.value = "";
  };

  const submit = async () => {
    if (!file) return;
    setPending(true);
    setError(false);
    try {
      const dataBase64 = await fileToBase64(file);
      const response = await imageLookup(dataBase64, file.name, similar, coversOnly, expunged);
      onResults({ response, filename: file.name });
      onOpenChange(false);
      reset();
    } catch {
      setError(true);
    } finally {
      setPending(false);
    }
  };

  const option = (
    label: string,
    hint: string,
    checked: boolean,
    onChange: (v: boolean) => void,
  ) => (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="min-w-0">
        <p className="text-base">{label}</p>
        <p className="text-caption leading-4 text-faint-foreground">{hint}</p>
      </div>
      <Switch checked={checked} onCheckedChange={onChange} aria-label={label} />
    </div>
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("imagesearch.title")}>
      <input
        ref={fileRef}
        type="file"
        accept="image/*"
        className="hidden"
        aria-label={t("imagesearch.pickAria")}
        onChange={(e) => {
          setFile(e.target.files?.[0] ?? null);
          setError(false);
        }}
      />
      <div className="space-y-4">
        <button
          onClick={() => fileRef.current?.click()}
          className="flex w-full items-center gap-3 rounded-lg border border-dashed border-border bg-surface p-3 text-left transition-colors duration-150 hover:border-accent/40"
        >
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-surface-2 text-muted-foreground">
            <ImagePlus size={16} strokeWidth={1.75} />
          </span>
          <span className="min-w-0">
            <span className="block truncate text-base font-medium">
              {file ? t("imagesearch.fileChosen", { name: file.name }) : t("imagesearch.chooseFile")}
            </span>
            <span className="block text-caption text-faint-foreground">
              {file ? file.name : t("imagesearch.noFile")}
            </span>
          </span>
        </button>

        <div className="divide-y divide-border">
          {option(t("imagesearch.similar"), t("imagesearch.similarHint"), similar, setSimilar)}
          {option(t("imagesearch.coversOnly"), t("imagesearch.coversOnlyHint"), coversOnly, setCoversOnly)}
          {option(t("imagesearch.includeExpunged"), t("imagesearch.includeExpungedHint"), expunged, setExpunged)}
        </div>

        {error && <p className="text-small text-danger">{t("imagesearch.errorDesc")}</p>}

        <div className="flex justify-end gap-2">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button size="sm" disabled={!file || pending} onClick={() => void submit()}>
            {pending ? t("imagesearch.searching") : t("imagesearch.submit")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
