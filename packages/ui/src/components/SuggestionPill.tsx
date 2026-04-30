// Phase 4.7 — SuggestionPill.
//
// Inline pill chip rendered next to (or over) the suggested-marked
// range in the editor surface. Shows the author + replacement
// preview + accept / reject buttons. Mirrors Word / Google Docs
// inline track-changes UI.
//
// Pure presentational: accept / reject callbacks are wired by the
// parent (typically `<SuggestionsController />`); this component
// owns no state.

import { type Component } from 'solid-js';

export type SuggestionState = 'pending' | 'accepted' | 'rejected';

export interface SuggestionPillProps {
  /** Suggestion id (used for ARIA + click-target identification). */
  id: string;
  author: string;
  /** Replacement text the suggestion would apply on accept. */
  replacement: string;
  /** Lifecycle state — controls button disabled state. */
  state: SuggestionState;
  onAccept?: ((id: string) => void) | undefined;
  onReject?: ((id: string) => void) | undefined;
  class?: string | undefined;
}

const articleClass =
  'inline-flex items-center gap-2 rounded border border-amber-300 bg-amber-50 px-2 py-1 text-xs shadow-sm dark:border-amber-700 dark:bg-amber-950';

const authorClass = 'font-semibold text-neutral-800 dark:text-neutral-100';

const replacementClass = 'font-mono text-amber-900 dark:text-amber-100';

const buttonClass =
  'rounded px-1.5 py-0.5 text-xs font-medium transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 disabled:cursor-not-allowed disabled:opacity-50';

const acceptClass = `${buttonClass} bg-emerald-600 text-white hover:bg-emerald-700`;

const rejectClass =
  `${buttonClass} border border-neutral-300 text-neutral-700 hover:bg-neutral-100 dark:border-neutral-700 dark:text-neutral-200 dark:hover:bg-neutral-800`;

export const SuggestionPill: Component<SuggestionPillProps> = (props) => {
  const isResolved = () => props.state !== 'pending';
  return (
    <article
      class={`${articleClass} ${props.class ?? ''}`.trim()}
      data-suggestion-id={props.id}
      aria-label={`Suggestion by ${props.author}`}
    >
      <span class={authorClass}>{props.author}</span>
      <span class={replacementClass}>{props.replacement}</span>
      <button
        type='button'
        class={acceptClass}
        disabled={isResolved()}
        onClick={() => props.onAccept?.(props.id)}
      >
        Accept
      </button>
      <button
        type='button'
        class={rejectClass}
        disabled={isResolved()}
        onClick={() => props.onReject?.(props.id)}
      >
        Reject
      </button>
    </article>
  );
};
