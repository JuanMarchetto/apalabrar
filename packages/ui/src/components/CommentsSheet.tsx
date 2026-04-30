// Phase 4.6 — CommentsSheet.
//
// Side-anchored sheet built on kobalte's `Dialog` primitive (kobalte
// has no dedicated `Sheet`; the side-panel pattern uses Dialog with
// custom slide-in styling). Wraps any children — typically a
// `<CommentsSidebar />` — and adds a backdrop + dismissible behavior
// (Esc, click-outside) inherited from kobalte Dialog.
//
// Accessibility: `aria-label` is set on Dialog.Content so AT users
// hear the sheet announced as "Comments" (or the caller-provided
// label). We deliberately don't render a visible title bar — the
// inner `<CommentsSidebar />` already exposes its own
// `aria-label="Comments"` aside, and a duplicate title would be
// announced twice.

import { Dialog } from '@kobalte/core/dialog';
import { type Component, type JSX } from 'solid-js';

export interface CommentsSheetProps {
  /** Controlled open state. */
  open: boolean;
  /** Fires when the user dismisses (Esc / click-outside / explicit close). */
  onOpenChange?: ((open: boolean) => void) | undefined;
  /** Sheet body — typically `<CommentsSidebar />`. */
  children: JSX.Element;
  /** Accessible label for AT users. Defaults to "Comments". */
  label?: string | undefined;
  class?: string | undefined;
}

const overlayClass =
  'fixed inset-0 z-40 bg-neutral-900/40 backdrop-blur-sm data-[expanded]:animate-in data-[closed]:animate-out';

const contentBaseClass =
  'fixed inset-y-0 right-0 z-50 flex w-full max-w-md flex-col bg-white shadow-xl outline-none dark:bg-neutral-900';

export const CommentsSheet: Component<CommentsSheetProps> = (props) => {
  return (
    <Dialog
      open={props.open}
      onOpenChange={(open) => props.onOpenChange?.(open)}
    >
      <Dialog.Portal>
        <Dialog.Overlay class={overlayClass} />
        <Dialog.Content
          class={`${contentBaseClass} ${props.class ?? ''}`.trim()}
          aria-label={props.label ?? 'Comments'}
        >
          {props.children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog>
  );
};
