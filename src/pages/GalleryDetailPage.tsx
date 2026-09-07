import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "react-router-dom";
import { BookOpen, Download, StickyNote } from "lucide-react";
import {
  addDownload,
  checkSignIn,
  getFavoriteNote,
  getGalleryDetail,
  isNotSignedIn,
  modifyFavoriteNote,
  rateGallery,
  recordHistory,
  tagDbStatus,
  toggleFavorite,
  translateTags,
} from "../lib/api";
import { readProgress } from "../lib/reading-progress";
import { useToastedMutation } from "../lib/use-toasted-mutation";
import { currentLocale, useT } from "../lib/i18n";
import { Button, GhostLink } from "../components/ui/button";
import { ErrorState } from "../components/ui/error-state";
import { RatingHearts } from "../components/gallery/rating-hearts";
import { RatingDialog } from "../components/gallery/rating-dialog";
import { GalleryHeader } from "../components/gallery/gallery-header";
import { FavoriteSlotPicker } from "../components/gallery/favorite-slot-picker";
import { FavoriteNoteDialog } from "../components/gallery/favorite-note-dialog";
import { TagSection } from "../components/gallery/tag-section";
import { DownloadOptionsSection } from "../components/gallery/download-options-section";
import { CommentsSection } from "../components/gallery/comments-section";
import { PreviewPager } from "../components/gallery/preview-pager";
import { DetailSkeleton } from "../components/gallery/detail-skeleton";

export function GalleryDetailPage() {
  const { gid = "0", token = "" } = useParams();
  const gidNum = Number(gid);
  const navigate = useNavigate();
  const t = useT();

  const query = useQuery({
    queryKey: ["gallery-detail", gidNum, token],
    queryFn: async () => {
      const detail = await getGalleryDetail(gidNum, token);
      await recordHistory(detail);
      return detail;
    },
    staleTime: 60_000,
  });

  const favMutation = useToastedMutation({
    mutationFn: (slot: number) => toggleFavorite(gidNum, token, slot),
    successMessage: (result) =>
      result.isFavorited ? t("detail.addedFav") : t("detail.removedFav"),
    successDescription: (result) =>
      result.isFavorited ? t("detail.slot", { n: result.slot + 1 }) : undefined,
    errorMessage: t("detail.favUpdateFailed"),
    invalidateKeys: [["gallery-detail", gidNum, token], ["favorites"]],
  });

  const rateMutation = useToastedMutation({
    mutationFn: (rating: number) => rateGallery(gidNum, token, rating),
    successMessage: t("detail.ratingSubmitted"),
    successDescription: (_result, rating) =>
      t("rating.yourRating", { n: Math.round(rating * 2) }),
    errorMessage: t("detail.ratingFailed"),
    invalidateKeys: [["gallery-detail", gidNum, token]],
    onSuccess: () => setRatingOpen(false),
  });

  const downloadMutation = useToastedMutation({
    mutationFn: () => addDownload(gidNum, token),
    successMessage: t("detail.addedToDownloads"),
    errorMessage: t("detail.addToDownloadsFailed"),
    invalidateKeys: [["downloads"]],
  });

  const isFavorited = !!query.data?.isFavorited;
  const signInQuery = useQuery({ queryKey: ["signin"], queryFn: checkSignIn });
  const [ratingOpen, setRatingOpen] = useState(false);
  const favNoteQuery = useQuery({
    queryKey: ["favorite-note", gidNum],
    queryFn: () => getFavoriteNote(gidNum),
    enabled: isFavorited,
    // Notes change rarely, and the lookup walks favorites pages server-side —
    // re-running it per mount is a request storm the site will punish.
    staleTime: Infinity,
    refetchOnMount: false,
    retry: 0,
  });
  const [favNoteOpen, setFavNoteOpen] = useState(false);
  const [favNoteDraft, setFavNoteDraft] = useState("");
  const favNoteMutation = useToastedMutation({
    mutationFn: (note: string) => modifyFavoriteNote(gidNum, token, note),
    successMessage: t("detail.favNoteSaved"),
    errorMessage: t("detail.favNoteFailed"),
    invalidateKeys: [["favorite-note", gidNum]],
    onSuccess: () => setFavNoteOpen(false),
  });

  const [favPickerOpen, setFavPickerOpen] = useState(false);

  // Tag translations: one batched request covering every tag group, gated on
  // the tag DB being available and the effective UI locale being zh-CN.
  // `useT` re-renders this page on language change, so `enabled` re-evaluates.
  const tagList = (query.data?.tags ?? []).flatMap((g) =>
    g.tags.map((name) => ({ namespace: g.namespace, name })),
  );
  const tagDbQuery = useQuery({ queryKey: ["tag-db-status"], queryFn: tagDbStatus });
  const tagTranslationsQuery = useQuery({
    queryKey: ["tag-translations", gidNum],
    queryFn: () => translateTags(tagList),
    enabled: tagList.length > 0 && !!tagDbQuery.data?.available && currentLocale() === "zh-CN",
  });
  const translationByTag = useMemo(() => {
    const map = new Map<string, string | null>();
    const translations = tagTranslationsQuery.data;
    if (!translations) return map;
    tagList.forEach((tag, i) => {
      map.set(`${tag.namespace}:${tag.name}`, translations[i]);
    });
    return map;
  }, [tagList, tagTranslationsQuery.data]);
  const translatedTag = (namespace: string, name: string): string | null =>
    translationByTag.get(`${namespace}:${name}`) ?? null;

  if (query.isLoading) return <DetailSkeleton />;
  if (query.isError || !query.data) {
    return (
      <div className="p-6">
        <ErrorState
          description={
            isNotSignedIn(query.error)
              ? t("detail.signInRequired")
              : t("detail.loadFailed")
          }
          onRetry={() => void query.refetch()}
          action={
            isNotSignedIn(query.error) ? (
              <GhostLink to="/signin">{t("common.signIn")}</GhostLink>
            ) : undefined
          }
        />
      </div>
    );
  }

  const d = query.data;
  const progress = Math.min(readProgress(gidNum), Math.max(0, d.pages - 1));

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-6xl p-4 pb-8 sm:p-6">
        <GalleryHeader d={d}>
          {/* rating + actions */}
          <div className="flex flex-wrap items-center gap-4">
            <div className="flex items-center gap-2">
              <button
                onClick={() => setRatingOpen(true)}
                aria-label={t("rating.title")}
                className="rounded p-0.5 transition-colors hover:text-accent"
              >
                <RatingHearts value={d.myRating || d.rating} />
              </button>
              <span className="font-mono text-caption text-faint-foreground tabular-nums">
                {d.rating.toFixed(1)} / 5
              </span>
              {d.myRating > 0 && (
                <span className="text-caption text-muted-foreground tabular-nums">
                  {t("rating.yourRating", { n: Math.round(d.myRating * 2) })}
                </span>
              )}
            </div>
          </div>

          <RatingDialog
            open={ratingOpen}
            onOpenChange={setRatingOpen}
            initialStars={Math.round(d.myRating * 2)}
            pending={rateMutation.isPending}
            onSubmit={(stars) => rateMutation.mutate(stars / 2)}
          />

          <div className="flex flex-wrap items-center gap-2">
            <Button
              onClick={() => navigate(`/read/${d.gid}/${d.token}`)}
              className="gap-1.5"
            >
              <BookOpen size={15} strokeWidth={1.75} />
              {progress > 0 ? t("detail.continue", { n: progress + 1 }) : t("detail.readNow")}
            </Button>
            {progress > 0 && <Button variant="ghost" onClick={() => navigate(`/read/${d.gid}/${d.token}?p=0`)}>{t("detail.startOver")}</Button>}
            <Button
              variant="subtle"
              onClick={() => downloadMutation.mutate()}
              disabled={downloadMutation.isPending}
              className="gap-1.5"
            >
              <Download size={15} strokeWidth={1.75} />
              {t("detail.addToDownloads")}
            </Button>
            {d.isFavorited && (
              <Button
                variant="outline"
                onClick={() => {
                  setFavNoteDraft(favNoteQuery.data ?? "");
                  setFavNoteOpen(true);
                }}
                className="gap-1.5"
              >
                <StickyNote size={15} strokeWidth={1.75} />
                {t("detail.favNote")}
              </Button>
            )}
            <FavoriteSlotPicker
              isFavorited={d.isFavorited}
              favoriteSlot={d.favoriteSlot}
              open={favPickerOpen}
              onOpenChange={setFavPickerOpen}
              onPick={(slot) => {
                setFavPickerOpen(false);
                favMutation.mutate(slot);
              }}
            />
          </div>
        </GalleryHeader>

        <FavoriteNoteDialog
          open={favNoteOpen}
          onOpenChange={setFavNoteOpen}
          draft={favNoteDraft}
          onDraftChange={setFavNoteDraft}
          saving={favNoteMutation.isPending}
          onSave={() => favNoteMutation.mutate(favNoteDraft.trim())}
        />

        <TagSection tags={d.tags} translated={translatedTag} />

        <DownloadOptionsSection
          gid={d.gid}
          token={d.token}
          torrentCount={d.torrentCount}
          signedIn={!!signInQuery.data}
        />

        {/* previews */}
        <section aria-label={t("detail.previews")} className="mt-8">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-emphasis font-semibold tracking-tight">
              {t("detail.previews")}{" "}
              <span className="font-normal text-faint-foreground">
                {t("detail.pageCount", { n: d.previewSet.total })}
              </span>
            </h3>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => navigate(`/read/${d.gid}/${d.token}`)}
            >
              {t("detail.viewAll")}
            </Button>
          </div>
          <PreviewPager gid={d.gid} token={d.token} total={d.pages} initial={d.previewSet} />
        </section>

        <CommentsSection
          gid={gidNum}
          token={token}
          comments={d.comments}
          commentCount={d.commentCount}
          hasMoreComments={d.hasMoreComments}
        />
      </div>
    </div>
  );
}
