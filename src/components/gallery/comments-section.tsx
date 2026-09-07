import { useEffect, useState } from "react";
import { MessageSquare, Send, ThumbsDown, ThumbsUp } from "lucide-react";
import { getComments, sendComment, voteComment } from "../../lib/api";
import type { Comment } from "../../lib/types";
import { useT } from "../../lib/i18n";
import { formatRelativeTime } from "../../lib/utils";
import { toast } from "../../stores/toast";
import { useToastedMutation } from "../../lib/use-toasted-mutation";
import { Button } from "../ui/button";
import { Textarea } from "../ui/textarea";
import { Spinner } from "../ui/spinner";
import { CommentBody } from "./comment-body";

/**
 * Comment list for the gallery detail page. Extra pages are fetched through
 * "load more" (pages are 1-based; the detail response already contains
 * page 1) and the extra state resets whenever the gallery changes. Votes
 * update the displayed score optimistically from the value the backend
 * returns, keyed by comment id.
 */
export function CommentsSection({
  gid,
  token,
  comments,
  commentCount,
  hasMoreComments,
}: {
  gid: number;
  token: string;
  comments: Comment[];
  commentCount: number;
  hasMoreComments: boolean;
}) {
  const t = useT();
  const [extraComments, setExtraComments] = useState<Comment[]>([]);
  const [nextCommentPage, setNextCommentPage] = useState(2);
  const [commentsExhausted, setCommentsExhausted] = useState(false);
  const [loadingMoreComments, setLoadingMoreComments] = useState(false);
  const [scoreOverrides, setScoreOverrides] = useState<Record<number, number>>({});
  const [votedComments, setVotedComments] = useState<Record<number, 1 | -1>>({});
  const [votingComment, setVotingComment] = useState<number | null>(null);
  const [draft, setDraft] = useState("");

  useEffect(() => {
    setExtraComments([]);
    setNextCommentPage(2);
    setCommentsExhausted(false);
    setScoreOverrides({});
    setVotedComments({});
    setDraft("");
  }, [gid, token]);

  const loadMoreComments = async () => {
    setLoadingMoreComments(true);
    try {
      const more = await getComments(gid, token, nextCommentPage);
      if (more.length === 0) setCommentsExhausted(true);
      else setExtraComments((prev) => [...prev, ...more]);
      setNextCommentPage((p) => p + 1);
    } catch {
      toast.error(t("common.unknownError"));
    } finally {
      setLoadingMoreComments(false);
    }
  };

  const vote = async (commentId: number, v: 1 | -1) => {
    setVotingComment(commentId);
    try {
      const score = await voteComment(gid, token, commentId, v);
      setScoreOverrides((prev) => ({ ...prev, [commentId]: score }));
      setVotedComments((prev) => ({ ...prev, [commentId]: v }));
    } catch {
      toast.error(t("comment.voteFailed"));
    } finally {
      setVotingComment(null);
    }
  };

  const sendMutation = useToastedMutation({
    mutationFn: (text: string) => sendComment(gid, token, text),
    successMessage: t("comment.posted"),
    errorMessage: t("comment.postFailed"),
    invalidateKeys: [["gallery-detail", gid, token]],
    onSuccess: () => setDraft(""),
  });

  const allComments = [...comments, ...extraComments];
  const scoreOf = (c: Comment) => scoreOverrides[c.id] ?? c.score;

  return (
    <section aria-label={t("detail.commentsSection")} className="mt-8">
      <h3 className="mb-3 flex items-center gap-1.5 text-emphasis font-semibold tracking-tight">
        <MessageSquare size={14} strokeWidth={1.75} className="text-muted-foreground" />
        {t("detail.comments", { n: commentCount || allComments.length })}
      </h3>
      {allComments.length === 0 ? (
        <p className="text-small text-faint-foreground">{t("detail.noComments")}</p>
      ) : (
        <>
          <ul className="max-w-[65ch] space-y-3">
            {allComments.map((c) => (
              <li key={c.id} className="rounded-lg border border-border bg-surface p-3.5">
                <div className="mb-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-caption">
                  <span className="font-medium text-foreground">{c.author}</span>
                  <span className="text-faint-foreground">{formatRelativeTime(c.time)}</span>
                  {c.lastEdited > 0 && (
                    <span className="text-faint-foreground">({t("comment.edited")})</span>
                  )}
                  <span className="ml-auto flex shrink-0 items-center gap-1 font-mono text-faint-foreground tabular-nums">
                    <button
                      onClick={() => void vote(c.id, 1)}
                      disabled={votingComment === c.id || c.id in votedComments}
                      aria-label={t("comment.voteUp")}
                      aria-pressed={votedComments[c.id] === 1}
                      className="rounded p-0.5 transition-colors hover:text-accent disabled:pointer-events-none disabled:opacity-50"
                    >
                      <ThumbsUp
                        size={11}
                        strokeWidth={1.75}
                        className={votedComments[c.id] === 1 ? "fill-accent text-accent" : ""}
                      />
                    </button>
                    {scoreOf(c)}
                    <button
                      onClick={() => void vote(c.id, -1)}
                      disabled={votingComment === c.id || c.id in votedComments}
                      aria-label={t("comment.voteDown")}
                      aria-pressed={votedComments[c.id] === -1}
                      className="rounded p-0.5 transition-colors hover:text-danger disabled:pointer-events-none disabled:opacity-50"
                    >
                      <ThumbsDown
                        size={11}
                        strokeWidth={1.75}
                        className={votedComments[c.id] === -1 ? "fill-danger text-danger" : ""}
                      />
                    </button>
                  </span>
                </div>
                <CommentBody text={c.text} />
              </li>
            ))}
          </ul>
          {hasMoreComments && !commentsExhausted && (
            <Button
              variant="outline"
              size="sm"
              className="mt-3"
              disabled={loadingMoreComments}
              onClick={() => void loadMoreComments()}
            >
              {t("comments.loadMore")}
            </Button>
          )}
        </>
      )}

      <form
        className="mt-4 max-w-[65ch]"
        onSubmit={(e) => {
          e.preventDefault();
          const text = draft.trim();
          if (text) sendMutation.mutate(text);
        }}
      >
        <Textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder={t("comment.replyPlaceholder")}
          aria-label={t("comment.replyPlaceholder")}
          rows={3}
          className="min-h-16"
        />
        <div className="mt-2 flex justify-end">
          <Button
            type="submit"
            size="sm"
            disabled={sendMutation.isPending || !draft.trim()}
            className="gap-1.5"
          >
            {sendMutation.isPending ? <Spinner size={13} /> : <Send size={13} strokeWidth={1.75} />}
            {t("comment.submit")}
          </Button>
        </div>
      </form>
    </section>
  );
}
