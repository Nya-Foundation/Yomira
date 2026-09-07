import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  Download,
  FolderDown,
  ListChecks,
  Pause,
  Play,
  Plus,
  Trash2,
  X
} from "lucide-react";
import { useState } from "react";
import { NewLabelDialog, RootDialog } from "../components/downloads/download-dialogs";
import { DownloadRow } from "../components/downloads/download-row";
import { Button } from "../components/ui/button";
import { Dialog } from "../components/ui/dialog";
import { EmptyState } from "../components/ui/empty-state";
import { ErrorState } from "../components/ui/error-state";
import { FilterChip } from "../components/ui/filter-chip";
import { Select } from "../components/ui/select";
import {
  addDownloadLabel,
  deleteDownloadLabel,
  downloadLabels,
  getDownloadRoot,
  listDownloads,
  pauseDownload,
  removeDownload,
  setDownloadLabel,
  setDownloadRoot,
  startDownload
} from "../lib/api";
import { useT, type TKey } from "../lib/i18n";
import type { DownloadEntry } from "../lib/types";
import { useDownloadEvents } from "../lib/use-download-events";
import { useToastedMutation } from "../lib/use-toasted-mutation";
import { toast } from "../stores/toast";

const ALL_DOWNLOAD_KEYS = [["downloads"], ["download-labels"], ["download-root"]];

interface RemoveTarget {
  entry: DownloadEntry;
  deleteFiles: boolean;
}

export function DownloadsPage() {
  const t = useT();
  const queryClient = useQueryClient();
  const [labelFilter, setLabelFilter] = useState<string | null>(null);
  const [newLabelOpen, setNewLabelOpen] = useState(false);
  const [rootDialogOpen, setRootDialogOpen] = useState(false);
  const [removeTarget, setRemoveTarget] = useState<RemoveTarget | null>(null);
  const [selectMode, setSelectMode] = useState(false);
  const [selected, setSelected] = useState<ReadonlySet<number>>(new Set());
  const [batchBusy, setBatchBusy] = useState(false);
  const [batchRemoveOpen, setBatchRemoveOpen] = useState(false);
  const [batchDeleteFiles, setBatchDeleteFiles] = useState(false);

  const downloadsQuery = useQuery({
    queryKey: ["downloads"],
    queryFn: listDownloads,
    // Fallback while anything is actively downloading; the event listener
    // below is the primary refresh path and idles to no polling otherwise.
    refetchInterval: (q) =>
      q.state.data?.some((d) => d.state === "downloading") ? 2000 : false,
  });

  const labelsQuery = useQuery({ queryKey: ["download-labels"], queryFn: downloadLabels });
  const rootQuery = useQuery({ queryKey: ["download-root"], queryFn: getDownloadRoot });

  useDownloadEvents(() => {
    void queryClient.invalidateQueries({ queryKey: ["downloads"] });
  });


  const startMutation = useToastedMutation({
    mutationFn: startDownload,
    successMessage: t("downloads.started"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
  });
  const pauseMutation = useToastedMutation({
    mutationFn: pauseDownload,
    successMessage: t("downloads.paused"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
  });
  const removeMutation = useToastedMutation({
    mutationFn: ({ entry, deleteFiles }: RemoveTarget) => removeDownload(entry.gid, deleteFiles),
    successMessage: t("downloads.removed"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
    onSuccess: () => setRemoveTarget(null),
  });
  const assignLabelMutation = useToastedMutation({
    mutationFn: ({ gid, label }: { gid: number; label: string | null }) => setDownloadLabel(gid, label),
    successMessage: t("downloads.labelAssigned"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
  });
  const newLabelMutation = useToastedMutation({
    mutationFn: addDownloadLabel,
    successMessage: t("downloads.labelCreated"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
    onSuccess: () => setNewLabelOpen(false),
  });
  const deleteLabelMutation = useToastedMutation({
    mutationFn: deleteDownloadLabel,
    successMessage: t("downloads.labelDeleted"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
  });
  const setRootMutation = useToastedMutation({
    mutationFn: setDownloadRoot,
    successMessage: t("downloads.rootChanged"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: ALL_DOWNLOAD_KEYS,
    onSuccess: () => setRootDialogOpen(false),
  });

  const labels = labelsQuery.data ?? [];
  const all = downloadsQuery.data ?? [];
  const entries = labelFilter === null ? all : all.filter((d) => d.label === labelFilter);

  const toggleSelected = (gid: number) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(gid)) next.delete(gid);
      else next.add(gid);
      return next;
    });

  const visibleGids = entries.map((d) => d.gid);
  const allVisibleSelected =
    visibleGids.length > 0 && visibleGids.every((g) => selected.has(g));

  const toggleSelectAll = () =>
    setSelected(allVisibleSelected ? new Set() : new Set(visibleGids));

  const exitSelectMode = () => {
    setSelectMode(false);
    setSelected(new Set());
  };

  /** Run one API call per selected gid, then toast a single summary. */
  const runBatch = async (
    fn: (gid: number) => Promise<void>,
    successKey: TKey,
  ) => {
    const gids = [...selected];
    if (gids.length === 0 || batchBusy) return;
    setBatchBusy(true);
    let failed = 0;
    await Promise.all(
      gids.map(async (gid) => {
        try {
          await fn(gid);
        } catch {
          failed++;
        }
      }),
    );
    setBatchBusy(false);
    if (failed > 0) {
      toast.error(t("downloads.batchFailed", { n: failed, total: gids.length }));
    } else {
      toast.success(t(successKey, { n: gids.length }));
    }
    for (const key of ALL_DOWNLOAD_KEYS) {
      void queryClient.invalidateQueries({ queryKey: key });
    }
    exitSelectMode();
  };

  return (
    <div className="flex h-full flex-col">
      {/* toolbar */}
      <div className="flex shrink-0 flex-col gap-2 border-b border-border px-4 pb-3 pt-3">
        <div className="flex items-center gap-1.5 overflow-x-auto pb-0.5">
          <span className="shrink-0 text-small text-faint-foreground">{t("downloads.labels")}</span>
          {labelsQuery.isError ? (
            <InlineQueryError
              description={t("downloads.labelsError")}
              onRetry={() => void labelsQuery.refetch()}
            />
          ) : (
            <>
              <span className="shrink-0">
                <FilterChip active={labelFilter === null} onClick={() => setLabelFilter(null)} label={t("common.all")} />
              </span>
              {labels.map((label) => (
                <span key={label} className="group/chip flex shrink-0 items-center">
                  <FilterChip active={labelFilter === label} onClick={() => setLabelFilter(label)} label={label} />
                  <button
                    onClick={() => deleteLabelMutation.mutate(label)}
                    aria-label={t("downloads.labelDelete", { name: label })}
                    className="rounded-full p-0.5 text-faint-foreground opacity-0 transition-opacity hover:text-danger group-hover/chip:opacity-100"
                  >
                    <X size={11} />
                  </button>
                </span>
              ))}
            </>
          )}
          <button
            onClick={() => setNewLabelOpen(true)}
            className="flex shrink-0 items-center gap-1 rounded-full border border-dashed border-border px-2.5 py-1 text-small text-muted-foreground transition-colors duration-150 hover:border-accent hover:text-accent"
          >
            <Plus size={12} />
            {t("downloads.newLabel")}
          </button>
          <Button
            variant={selectMode ? "default" : "outline"}
            size="sm"
            onClick={() => (selectMode ? exitSelectMode() : setSelectMode(true))}
            aria-label={t("downloads.selectMode")}
            className="ml-auto shrink-0 gap-1.5"
          >
            <ListChecks size={14} strokeWidth={1.75} />
            {t("downloads.selectMode")}
          </Button>
        </div>
        {selectMode && (
          <div className="flex items-center gap-2 overflow-x-auto pb-0.5">
            <label className="flex shrink-0 cursor-pointer items-center gap-1.5 text-small text-muted-foreground select-none">
              <input
                type="checkbox"
                checked={allVisibleSelected}
                onChange={toggleSelectAll}
                className="h-3.5 w-3.5 accent-accent"
                aria-label={t("downloads.selectAll")}
              />
              {t("downloads.selectAll")}
            </label>
            <span className="shrink-0 text-caption text-faint-foreground tabular-nums">
              {t("downloads.selected", { n: selected.size })}
            </span>
            {selected.size > 0 && (
              <div className="ml-auto flex shrink-0 items-center gap-1.5">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={batchBusy}
                  onClick={() => void runBatch(startDownload, "downloads.batchStarted")}
                  aria-label={t("downloads.batchStart")}
                  className="gap-1.5"
                >
                  <Play size={13} strokeWidth={1.75} />
                  {t("downloads.batchStart")}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={batchBusy}
                  onClick={() => void runBatch(pauseDownload, "downloads.batchPaused")}
                  aria-label={t("downloads.batchPause")}
                  className="gap-1.5"
                >
                  <Pause size={13} strokeWidth={1.75} />
                  {t("downloads.batchPause")}
                </Button>
                <Select
                  value="__placeholder"
                  onChange={(e) => {
                    const label = e.target.value === "" ? null : e.target.value;
                    if (e.target.value === "__placeholder") return;
                    void runBatch(
                      (gid) => setDownloadLabel(gid, label),
                      "downloads.batchLabeled",
                    );
                  }}
                  disabled={batchBusy}
                  aria-label={t("downloads.assignLabel")}
                  className="w-32"
                >
                  <option value="__placeholder" disabled>
                    {t("downloads.assignLabel")}
                  </option>
                  <option value="">{t("downloads.noLabel")}</option>
                  {labels.map((label) => (
                    <option key={label} value={label}>
                      {label}
                    </option>
                  ))}
                </Select>
                <Button
                  variant="ghost"
                  size="icon"
                  disabled={batchBusy}
                  onClick={() => setBatchRemoveOpen(true)}
                  aria-label={t("downloads.batchRemove")}
                  className="text-muted-foreground hover:text-danger"
                >
                  <Trash2 size={14} strokeWidth={1.75} />
                </Button>
              </div>
            )}
          </div>
        )}
        <div className="flex items-center gap-2">
          <FolderDown size={14} strokeWidth={1.75} className="shrink-0 text-faint-foreground" />
          <span className="shrink-0 text-small text-faint-foreground">{t("downloads.root")}</span>
          {rootQuery.isError ? (
            <InlineQueryError
              description={t("downloads.rootError")}
              onRetry={() => void rootQuery.refetch()}
            />
          ) : (
            <span className="min-w-0 flex-1 truncate font-mono text-small text-muted-foreground">
              {rootQuery.data ?? ""}
            </span>
          )}
          <Button variant="outline" size="sm" onClick={() => setRootDialogOpen(true)}>
            {t("downloads.changeRoot")}
          </Button>
        </div>
      </div>

      {/* list */}
      {downloadsQuery.isLoading ? (
        <div className="min-h-0 flex-1 space-y-3 overflow-hidden p-4">
          {Array.from({ length: 4 }, (_, i) => (
            <div key={i} className="flex items-center gap-3">
              <div className="h-16 w-12 animate-pulse rounded-md bg-surface-2" />
              <div className="flex-1 space-y-2">
                <div className="h-4 w-2/3 animate-pulse rounded bg-surface-2" />
                <div className="h-2.5 w-1/2 animate-pulse rounded-full bg-surface-2" />
              </div>
            </div>
          ))}
        </div>
      ) : downloadsQuery.isError ? (
        <ErrorState
          description={t("downloads.loadError")}
          onRetry={() => void downloadsQuery.refetch()}
        />
      ) : entries.length === 0 ? (
        <EmptyState
          icon={Download}
          title={t("downloads.empty")}
          description={t("downloads.emptyDesc")}
        />
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          {entries.map((d) => (
            <DownloadRow
              key={d.gid}
              entry={d}
              labels={labels}
              busy={startMutation.isPending || pauseMutation.isPending || assignLabelMutation.isPending}
              selectMode={selectMode}
              selected={selected.has(d.gid)}
              onToggleSelect={() => toggleSelected(d.gid)}
              onStart={() => startMutation.mutate(d.gid)}
              onPause={() => pauseMutation.mutate(d.gid)}
              onRemove={() => setRemoveTarget({ entry: d, deleteFiles: false })}
              onAssignLabel={(label) => assignLabelMutation.mutate({ gid: d.gid, label })}
            />
          ))}
        </div>
      )}

      {/* new label dialog */}
      <NewLabelDialog
        open={newLabelOpen}
        onOpenChange={setNewLabelOpen}
        pending={newLabelMutation.isPending}
        onSubmit={(name) => newLabelMutation.mutate(name)}
      />

      {/* download root dialog */}
      <RootDialog
        open={rootDialogOpen}
        onOpenChange={setRootDialogOpen}
        current={rootQuery.data ?? ""}
        pending={setRootMutation.isPending}
        onSubmit={(path) => setRootMutation.mutate(path)}
      />

      {/* batch remove confirm */}
      {batchRemoveOpen && (
        <Dialog
          open
          onOpenChange={() => setBatchRemoveOpen(false)}
          title={t("downloads.batchRemoveTitle")}
          className="max-w-sm"
        >
          <p className="mb-3 text-base leading-6 text-muted-foreground">
            {t("downloads.batchRemoveDesc", { n: selected.size })}
          </p>
          <label className="mb-4 flex cursor-pointer items-center gap-2 text-small text-muted-foreground select-none">
            <input
              type="checkbox"
              checked={batchDeleteFiles}
              onChange={(e) => setBatchDeleteFiles(e.target.checked)}
              className="h-3.5 w-3.5 accent-accent"
            />
            {t("downloads.deleteFiles")}
          </label>
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setBatchRemoveOpen(false)}>
              {t("common.cancel")}
            </Button>
            <button
              onClick={() => {
                setBatchRemoveOpen(false);
                void runBatch(
                  (gid) => removeDownload(gid, batchDeleteFiles),
                  "downloads.batchRemoved",
                );
              }}
              disabled={batchBusy}
              className="h-7 rounded-md border border-danger/30 bg-danger/10 px-2.5 text-small font-medium text-danger transition-colors hover:bg-danger/20 disabled:pointer-events-none disabled:opacity-50"
            >
              {t("common.confirm")}
            </button>
          </div>
        </Dialog>
      )}

      {/* remove confirm */}
      {removeTarget && (
        <Dialog
          open
          onOpenChange={() => setRemoveTarget(null)}
          title={t("downloads.removeTitle")}
          className="max-w-sm"
        >
          <p className="mb-3 text-base leading-6 text-muted-foreground">
            {t("downloads.removeDesc", { title: removeTarget.entry.title })}
          </p>
          <label className="mb-4 flex cursor-pointer items-center gap-2 text-small text-muted-foreground select-none">
            <input
              type="checkbox"
              checked={removeTarget.deleteFiles}
              onChange={(e) =>
                setRemoveTarget((cur) => cur && { ...cur, deleteFiles: e.target.checked })
              }
              className="h-3.5 w-3.5 accent-accent"
            />
            {t("downloads.deleteFiles")}
          </label>
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setRemoveTarget(null)}>
              {t("common.cancel")}
            </Button>
            <button
              onClick={() => removeMutation.mutate(removeTarget)}
              disabled={removeMutation.isPending}
              className="h-7 rounded-md border border-danger/30 bg-danger/10 px-2.5 text-small font-medium text-danger transition-colors hover:bg-danger/20 disabled:pointer-events-none disabled:opacity-50"
            >
              {t("common.confirm")}
            </button>
          </div>
        </Dialog>
      )}
    </div>
  );
}

function InlineQueryError({ description, onRetry }: { description: string; onRetry: () => void }) {
  const t = useT();
  return (
    <span className="flex min-w-0 items-center gap-1.5 text-small text-danger">
      <AlertTriangle size={12} strokeWidth={1.75} className="shrink-0" />
      <span className="truncate">{description}</span>
      <button
        onClick={onRetry}
        className="shrink-0 rounded px-1 font-medium text-accent hover:underline"
      >
        {t("common.retry")}
      </button>
    </span>
  );
}
