/**
 * Phase 2.5 — "Continue last doc?" toast.
 *
 * Surfaces if `bootstrapOpfs` returns a non-empty `recents` list.
 * Auto-dismisses after `autoDismissMs` (default 5000) so it never
 * blocks a fresh-start flow. Dismissal is non-destructive — the
 * doc still lives in OPFS and the user can pick it from the
 * `RecentDocsMenu`.
 *
 * Accessibility:
 * - role="status" + aria-live="polite" so AT users hear the offer
 *   without being interrupted.
 * - The Continue and Dismiss buttons are real `<button>`s.
 */
import type { DocId, DocMetadata } from '@apalabrar/editor-bridge';
import { type Component, onCleanup, onMount, Show } from 'solid-js';

export interface ContinueToastProps {
  readonly docs: readonly DocMetadata[];
  readonly onContinue: (id: DocId) => void;
  readonly onDismiss: () => void;
  /** Auto-dismiss timeout in ms. Defaults to 5000 (blueprint spec). */
  readonly autoDismissMs?: number;
}

const DEFAULT_AUTO_DISMISS_MS = 5000;

export const ContinueToast: Component<ContinueToastProps> = (props) => {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let dismissed = false;

  const cancelTimer = () => {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  };

  const handleDismiss = () => {
    if (dismissed) return;
    dismissed = true;
    cancelTimer();
    props.onDismiss();
  };

  const handleContinue = (id: DocId) => {
    if (dismissed) return;
    dismissed = true;
    cancelTimer();
    props.onContinue(id);
  };

  onMount(() => {
    if (props.docs.length === 0) return;
    const ms = props.autoDismissMs ?? DEFAULT_AUTO_DISMISS_MS;
    timer = setTimeout(handleDismiss, ms);
  });

  onCleanup(cancelTimer);

  return (
    <Show when={props.docs.length > 0}>
      <div
        role='status'
        aria-live='polite'
        data-testid='continue-toast'
        class='fixed bottom-4 right-4 z-50 max-w-sm rounded-lg border border-neutral-200 bg-white p-4 shadow-lg'
      >
        <p class='text-sm text-neutral-900'>Continue last doc?</p>
        <p class='mt-1 text-xs text-neutral-500'>
          {props.docs.length === 1 ?
            '1 saved document' :
            `${props.docs.length} saved documents`}
        </p>
        <div class='mt-3 flex gap-2'>
          <button
            type='button'
            class='rounded-md bg-neutral-900 px-3 py-1.5 text-sm text-white hover:bg-neutral-700'
            onClick={() => handleContinue(props.docs[0]!.id)}
          >
            Continue
          </button>
          <button
            type='button'
            class='rounded-md border border-neutral-300 px-3 py-1.5 text-sm text-neutral-700 hover:bg-neutral-100'
            onClick={handleDismiss}
          >
            Dismiss
          </button>
        </div>
      </div>
    </Show>
  );
};
