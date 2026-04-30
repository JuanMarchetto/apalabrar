// Phase 4.6 — CommentsController.
//
// Wraps `<CommentsSheet>` + `<CommentsSidebar>` into a single
// integration point that:
//   - maps doc-model `Comment[]` (codepoint anchors) into the UI's
//     `CommentThread[]` shape (string anchorBlockId)
//   - dispatches `SetCommentStatus` / `ReplyToComment` EditOps via
//     the injected `applyEditOp` callback
//   - emits `onScrollToPosition(from)` when a thread is jumped to
//
// The applyEditOp dispatcher is INJECTED so component tests can pass
// a `vi.fn()` and assert dispatch shape without spinning up wasm.
//
// Anchor encoding: thread's codepoint `from` becomes the string
// `pos-{from}` in the sidebar's `anchorBlockId` slot. The controller
// is the only code path that produces that string, so the
// translation back to a number on jump-to-anchor is symmetric.

import type { Comment, EditOp } from '@apalabrar/editor-bridge';
import { CommentsSheet, CommentsSidebar, type CommentThread } from '@apalabrar/ui';
import { type Component } from 'solid-js';

export interface CommentsControllerProps {
  /** All threads currently in the doc (parent reads via `core.comments(id)`). */
  threads: ReadonlyArray<Comment>;
  /** Author name attributed to new replies. */
  author: string;
  /** Sheet open state (controlled by parent). */
  open: boolean;
  onOpenChange?: ((open: boolean) => void) | undefined;
  /**
   * Dispatch an EditOp through the wasm core. Bind in production as
   * `(op) => core.applyEditOp(docId, op)`.
   */
  applyEditOp: (op: EditOp) => void;
  /**
   * Fires when the user clicks a thread to jump to its anchor. The
   * argument is the codepoint `from` of the thread's anchor — the
   * parent (editor surface) is responsible for translating to a
   * scroll position via `layout::resolve_selection`.
   */
  onScrollToPosition?: ((position: number) => void) | undefined;
  /** Current epoch-millis source for new replies. Tests inject a stub. */
  now?: (() => number) | undefined;
}

const ANCHOR_PREFIX = 'pos-';

const toThread = (c: Comment): CommentThread => ({
  id: c.thread_id,
  author: c.author,
  body: c.body,
  createdAt: new Date(c.created_at).toISOString(),
  status: c.status,
  anchorBlockId: `${ANCHOR_PREFIX}${c.from}`,
  replies: c.replies.map((r) => ({
    id: r.id,
    author: r.author,
    body: r.body,
    createdAt: new Date(r.created_at).toISOString(),
  })),
});

const decodeAnchor = (id: string): number | null => {
  if (!id.startsWith(ANCHOR_PREFIX)) return null;
  const n = Number(id.slice(ANCHOR_PREFIX.length));
  return Number.isFinite(n) ? n : null;
};

export const CommentsController: Component<CommentsControllerProps> = (props) => {
  const threadsForSidebar = (): CommentThread[] => props.threads.map(toThread);

  const handleJumpToAnchor = (anchorBlockId: string) => {
    const pos = decodeAnchor(anchorBlockId);
    if (pos !== null) props.onScrollToPosition?.(pos);
  };

  const handleResolve = (threadId: string) => {
    props.applyEditOp({
      kind: 'SetCommentStatus',
      thread_id: threadId,
      status: 'resolved',
    });
  };

  const handleReply = (threadId: string, body: string) => {
    const now = props.now ?? Date.now;
    props.applyEditOp({
      kind: 'ReplyToComment',
      thread_id: threadId,
      body,
      author: props.author,
      created_at: now(),
    });
  };

  return (
    <CommentsSheet open={props.open} onOpenChange={props.onOpenChange}>
      <CommentsSidebar
        threads={threadsForSidebar()}
        onJumpToAnchor={handleJumpToAnchor}
        onResolve={handleResolve}
        onReply={handleReply}
      />
    </CommentsSheet>
  );
};
