// Comments sidebar. Lists threads with author / body / replies and fires
// callbacks for the three primary actions: jump-to-anchor, resolve, reply.
//
// Design choices:
//   - Each thread is an <article> so AT users can navigate by article
//     landmark within the sidebar.
//   - Reply form is rendered inline (only when onReply is provided) — a
//     full reply modal would block the writer's flow.
//   - We don't include a "create new thread" action here; that's owned
//     by the toolbar/selection-toolbar (creates a thread anchored on the
//     current selection, then the new thread shows up via props).

import { type Component, createSignal, For, Show } from 'solid-js';

export type CommentStatus = 'open' | 'resolved';

export interface CommentReply {
  id: string;
  author: string;
  body: string;
  /** ISO 8601 timestamp. */
  createdAt: string;
}

export interface CommentThread {
  id: string;
  author: string;
  body: string;
  /** ISO 8601 timestamp. */
  createdAt: string;
  status: CommentStatus;
  /** doc-model BlockId where the anchor lives. */
  anchorBlockId: string;
  replies?: ReadonlyArray<CommentReply>;
}

export interface CommentsSidebarProps {
  threads: ReadonlyArray<CommentThread>;
  activeThreadId?: string;
  onJumpToAnchor?: (blockId: string) => void;
  onResolve?: (threadId: string) => void;
  /** When omitted, the inline reply form is hidden. */
  onReply?: (threadId: string, body: string) => void;
  emptyMessage?: string;
  class?: string;
}

const rootClass = 'flex h-full w-full flex-col gap-3 overflow-y-auto p-3 text-sm';

const listClass = 'flex flex-col gap-3';

const articleClass =
  'rounded-lg border border-neutral-200 bg-white p-3 shadow-sm dark:border-neutral-800 dark:bg-neutral-900';

const articleActiveClass = 'border-blue-500 ring-2 ring-blue-500/20 dark:border-blue-400';

// `opacity-60` would knock effective text contrast below 4.5:1; we use
// a tint + left border instead to mark resolved without hurting a11y.
const articleResolvedClass = 'bg-neutral-50 border-l-4 border-l-emerald-500 dark:bg-neutral-800';

const headerClass = 'flex items-center justify-between gap-2 text-xs';

const authorClass = 'font-semibold text-neutral-900 dark:text-neutral-100';

const timeClass = 'text-neutral-600 dark:text-neutral-300';

const statusBadgeClass =
  'rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-900 dark:bg-emerald-900/40 dark:text-emerald-100';

const bodyClass = 'mt-2 whitespace-pre-wrap leading-relaxed text-neutral-800 dark:text-neutral-200';

const replyListClass =
  'mt-3 flex flex-col gap-2 border-l-2 border-neutral-200 pl-3 dark:border-neutral-700';

const replyArticleClass = 'text-xs';

const actionsClass = 'mt-3 flex items-center gap-2';

const buttonBaseClass =
  'rounded px-2 py-1 text-xs font-medium transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600';

const buttonSecondaryClass =
  'border border-neutral-300 bg-white text-neutral-700 hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-200 dark:hover:bg-neutral-800';

const buttonPrimaryClass =
  'bg-blue-600 text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50';

const replyFormClass = 'mt-3 flex flex-col gap-2';

const replyTextareaClass =
  'min-h-16 w-full resize-y rounded border border-neutral-300 p-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-neutral-700 dark:bg-neutral-900';

const emptyClass = 'px-2 py-8 text-center text-neutral-600 dark:text-neutral-300';

interface ReplyFormProps {
  threadId: string;
  onSubmit: (threadId: string, body: string) => void;
}

const ReplyForm: Component<ReplyFormProps> = (props) => {
  const [draft, setDraft] = createSignal('');
  return (
    <form
      class={replyFormClass}
      aria-label='Reply'
      onSubmit={(event) => {
        event.preventDefault();
        const body = draft().trim();
        if (body.length === 0) return;
        props.onSubmit(props.threadId, body);
        setDraft('');
      }}
    >
      <textarea
        class={replyTextareaClass}
        placeholder='Reply…'
        aria-label='Reply text'
        value={draft()}
        onInput={(event) => setDraft(event.currentTarget.value)}
      />
      <button
        type='submit'
        class={`${buttonBaseClass} ${buttonPrimaryClass} self-end`}
        disabled={draft().trim().length === 0}
      >
        Reply
      </button>
    </form>
  );
};

export const CommentsSidebar: Component<CommentsSidebarProps> = (props) => {
  return (
    <aside
      aria-label='Comments'
      class={`${rootClass} ${props.class ?? ''}`.trim()}
    >
      <Show
        when={props.threads.length > 0}
        fallback={<p class={emptyClass}>{props.emptyMessage ?? 'No comments yet.'}</p>}
      >
        <ul class={listClass} role='list'>
          <For each={props.threads}>
            {(thread) => {
              const isActive = () => props.activeThreadId === thread.id;
              const isResolved = () => thread.status === 'resolved';
              const articleClasses = () =>
                `${articleClass} ${isActive() ? articleActiveClass : ''} ${
                  isResolved() ? articleResolvedClass : ''
                }`.trim();
              return (
                <li>
                  <article
                    class={articleClasses()}
                    aria-current={isActive() ? 'true' : undefined}
                    data-thread-id={thread.id}
                  >
                    <header class={headerClass}>
                      <span class={authorClass}>{thread.author}</span>
                      <time class={timeClass} datetime={thread.createdAt}>
                        {thread.createdAt}
                      </time>
                    </header>
                    <Show when={isResolved()}>
                      <span class={statusBadgeClass} aria-label='Resolved'>
                        Resolved
                      </span>
                    </Show>
                    <p class={bodyClass}>{thread.body}</p>
                    <Show
                      when={thread.replies && thread.replies.length > 0}
                    >
                      <ul class={replyListClass} role='list'>
                        <For each={thread.replies}>
                          {(reply) => (
                            <li>
                              <article class={replyArticleClass}>
                                <header class={headerClass}>
                                  <span class={authorClass}>
                                    {reply.author}
                                  </span>
                                  <time
                                    class={timeClass}
                                    datetime={reply.createdAt}
                                  >
                                    {reply.createdAt}
                                  </time>
                                </header>
                                <p class={bodyClass}>{reply.body}</p>
                              </article>
                            </li>
                          )}
                        </For>
                      </ul>
                    </Show>
                    <div class={actionsClass}>
                      <button
                        type='button'
                        class={`${buttonBaseClass} ${buttonSecondaryClass}`}
                        onClick={() => props.onJumpToAnchor?.(thread.anchorBlockId)}
                      >
                        Jump to anchor
                      </button>
                      <button
                        type='button'
                        class={`${buttonBaseClass} ${buttonSecondaryClass}`}
                        disabled={isResolved()}
                        onClick={() => props.onResolve?.(thread.id)}
                      >
                        Resolve
                      </button>
                    </div>
                    <Show when={props.onReply}>
                      <ReplyForm
                        threadId={thread.id}
                        onSubmit={props.onReply!}
                      />
                    </Show>
                  </article>
                </li>
              );
            }}
          </For>
        </ul>
      </Show>
    </aside>
  );
};
