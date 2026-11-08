import { useQuery, useQueryClient } from "@tanstack/react-query";
import { PageToolbar, ScrollRow } from "../components/layout/page-chrome";
import { Skeleton } from "../components/ui/skeleton";
import {
  AlertTriangle,
  Download,
  Filter,
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
import { LabelSelect } from "../components/downloads/label-select";
import { Button } from "../components/ui/button";
import { CheckboxField } from "../components/ui/checkbox";
import { ConfirmDialog, Dialog } from "../components/ui/dialog";
import { EmptyState } from "../components/ui/empty-state";
import { ErrorState } from "../components/ui/error-state";
import { FilterChip } from "../components/ui/filter-chip";
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

const QUEUE = [["downloads"]];
/** Deleting a label also unlabels the entries that carried it. */
const QUEUE_AND_LABELS = [["downloads"], ["download-labels"]];
/** A new root changes which saved files the queue can find. */
const QUEUE_AND_ROOT = [["downloads"], ["download-root"]];

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
  const [labelToDelete, setLabelToDelete] = useState<string | null>(null);

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
    invalidateKeys: QUEUE,
  });
  const pauseMutation = useToastedMutation({
    mutationFn: pauseDownload,
    successMessage: t("downloads.paused"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: QUEUE,
  });
  const removeMutation = useToastedMutation({
    mutationFn: ({ entry, deleteFiles }: RemoveTarget) => removeDownload(entry.gid, deleteFiles),
    successMessage: t("downloads.removed"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: QUEUE,
    onSuccess: () => setRemoveTarget(null),
  });
  const assignLabelMutation = useToastedMutation({
    mutationFn: ({ gid, label }: { gid: number; label: string | null }) => setDownloadLabel(gid, label),
    successMessage: t("downloads.labelAssigned"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: QUEUE,
  });
  const newLabelMutation = useToastedMutation({
    mutationFn: addDownloadLabel,
    successMessage: t("downloads.labelCreated"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: [["download-labels"]],
    onSuccess: () => setNewLabelOpen(false),
  });
  const deleteLabelMutation = useToastedMutation({
    mutationFn: deleteDownloadLabel,
    successMessage: t("downloads.labelDeleted"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: QUEUE_AND_LABELS,
  });
  const setRootMutation = useToastedMutation({
    mutationFn: setDownloadRoot,
    successMessage: t("downloads.rootChanged"),
    errorMessage: t("downloads.actionFailed"),
    invalidateKeys: QUEUE_AND_ROOT,
    onSuccess: () => setRootDialogOpen(false),
  });

  // Only the row whose action is in flight; one global flag greyed out the
  // controls on every other download as well.
  const rowBusy = (gid: number) =>
    (startMutation.isPending && startMutation.variables === gid) ||
    (pauseMutation.isPending && pauseMutation.variables === gid) ||
    (assignLabelMutation.isPending && assignLabelMutation.variables?.gid === gid);

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
    for (const key of QUEUE) {
      void queryClient.invalidateQueries({ queryKey: key });
    }
    exitSelectMode();
  };

  return (
    <div className="flex h-full flex-col">
      {/* toolbar */}
      <PageToolbar>
        <ScrollRow label={t("downloads.labels")}>
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
                    onClick={() => setLabelToDelete(label)}
                    aria-label={t("downloads.labelDelete", { name: label })}
                    className="ml-1 rounded-full p-0.5 text-faint-foreground opacity-0 transition-opacity hover:text-danger group-hover/chip:opacity-100"
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
        </ScrollRow>
        {selectMode && (
          <ScrollRow className="gap-2">
            <CheckboxField
              className="shrink-0 gap-1.5"
              checked={allVisibleSelected}
              onChange={toggleSelectAll}
              aria-label={t("downloads.selectAll")}
              label={t("downloads.selectAll")}
            />
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
                <LabelSelect
                  labels={labels}
                  placeholder={t("downloads.assignLabel")}
                  disabled={batchBusy}
                  onChange={(label) =>
                    void runBatch((gid) => setDownloadLabel(gid, label), "downloads.batchLabeled")
                  }
                />
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
          </ScrollRow>
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
      </PageToolbar>

      {/* list */}
      {downloadsQuery.isLoading ? (
        <div className="min-h-0 flex-1 space-y-3 overflow-hidden p-4">
          {Array.from({ length: 4 }, (_, i) => (
            <div key={i} className="flex items-center gap-3">
              <Skeleton className="h-16 w-12" />
              <div className="flex-1 space-y-2">
                <Skeleton className="h-4 w-2/3 rounded" />
                <Skeleton className="h-2.5 w-1/2 rounded-full" />
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
        labelFilter !== null && all.length > 0 ? (
          <EmptyState
            icon={Filter}
            title={t("downloads.emptyFiltered")}
            description={t("downloads.emptyFilteredDesc", { n: all.length })}
            action={
              <Button variant="outline" size="sm" onClick={() => setLabelFilter(null)}>
                {t("downloads.showAll")}
              </Button>
            }
          />
        ) : (
          <EmptyState
            icon={Download}
            title={t("downloads.empty")}
            description={t("downloads.emptyDesc")}
          />
        )
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          {entries.map((d) => (
            <DownloadRow
              key={d.gid}
              entry={d}
              labels={labels}
              busy={rowBusy(d.gid)}
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

      {/* deleting a label also strips it from every entry carrying it */}
      <ConfirmDialog
        open={labelToDelete !== null}
        onOpenChange={(next) => !next && setLabelToDelete(null)}
        title={t("downloads.labelDeleteTitle")}
        description={t("downloads.labelDeleteDesc", {
          name: labelToDelete ?? "",
          n: all.filter((d) => d.label === labelToDelete).length,
        })}
        confirmLabel={t("common.delete")}
        onConfirm={() => {
          if (labelToDelete !== null) deleteLabelMutation.mutate(labelToDelete);
          setLabelToDelete(null);
        }}
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
          <CheckboxField
            className="mb-4"
            checked={batchDeleteFiles}
            onChange={(e) => setBatchDeleteFiles(e.target.checked)}
            label={t("downloads.deleteFiles")}
          />
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setBatchRemoveOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              size="sm"
              onClick={() => {
                setBatchRemoveOpen(false);
                void runBatch(
                  (gid) => removeDownload(gid, batchDeleteFiles),
                  "downloads.batchRemoved",
                );
              }}
              disabled={batchBusy}
            >
              {t("common.confirm")}
            </Button>
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
          <CheckboxField
            className="mb-4"
            checked={removeTarget.deleteFiles}
            onChange={(e) =>
              setRemoveTarget((cur) => cur && { ...cur, deleteFiles: e.target.checked })
            }
            label={t("downloads.deleteFiles")}
          />
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setRemoveTarget(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              size="sm"
              onClick={() => removeMutation.mutate(removeTarget)}
              disabled={removeMutation.isPending}
            >
              {t("common.confirm")}
            </Button>
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
