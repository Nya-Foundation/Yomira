
import { invokeOrThrow, isTauri, loadMock } from "./transport";

// ---------------------------------------------------------------------------
// Comment actions. `voteComment` resolves to the comment's updated score.
// ---------------------------------------------------------------------------

export async function voteComment(
  gid: number,
  token: string,
  commentId: number,
  vote: 1 | -1,
): Promise<number> {
  if (!isTauri()) return (await loadMock()).mock.voteComment(gid, commentId, vote);
  return invokeOrThrow("vote_comment", { gid, token, commentId, vote });
}

export async function sendComment(gid: number, token: string, text: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.sendComment(gid, text);
  return invokeOrThrow("send_comment", { gid, token, text });
}
