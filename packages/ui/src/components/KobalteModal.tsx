// Generic accessible modal wrapper around @kobalte/core/dialog.
//
// We expose a controlled API only: callers own `open` state and react to
// `onOpenChange`. The reasons:
//   - Editor commands (find/replace, comments, link insert) live outside
//     the modal body, so an uncontrolled trigger pattern doesn't fit.
//   - Tests need deterministic open/close without DOM events.
//
// Accessibility is delegated to kobalte: Title and Description are wired
// via aria-labelledby / aria-describedby on the dialog Content automatically.
// We only contribute layout + typography classes, never aria attrs.
//
// Visual styling uses Tailwind 4 utility classes resolved by the consuming
// app's index.css. Components in @apalabrar/ui never import CSS.

import { Dialog } from '@kobalte/core/dialog';
import { type Component, type JSX, Show } from 'solid-js';

export interface KobalteModalProps {
  /** Controlled open state. Closed when false. */
  open: boolean;
  /** Fired by user dismiss (close button, overlay click, ESC). */
  onOpenChange?: (open: boolean) => void;
  /** Visible header text — also wired as the dialog's accessible name. */
  title: JSX.Element;
  /** Optional supporting copy under the title. */
  description?: JSX.Element;
  /** Modal body. */
  children?: JSX.Element;
  /** Optional footer slot for action buttons (right-aligned by default). */
  footer?: JSX.Element;
  /** Extra classes for the content panel (e.g. width override). */
  contentClass?: string;
  /** Hide the built-in close button (e.g. for required-action modals). */
  hideCloseButton?: boolean;
}

const overlayClass =
  'fixed inset-0 z-40 bg-neutral-900/50 backdrop-blur-sm data-[expanded]:animate-in data-[closed]:animate-out';

const positionerClass = 'fixed inset-0 z-50 flex items-center justify-center p-4';

const contentBaseClass =
  'flex w-full max-w-lg flex-col gap-4 rounded-lg bg-white p-6 text-neutral-900 shadow-xl outline-none dark:bg-neutral-900 dark:text-neutral-100';

const headerClass = 'flex items-start justify-between gap-4';

const titleClass = 'text-lg font-semibold leading-tight';

const descriptionClass = 'text-sm text-neutral-700 dark:text-neutral-300';

const closeButtonClass =
  'inline-flex h-8 w-8 items-center justify-center rounded-md text-neutral-700 transition hover:bg-neutral-100 hover:text-neutral-900 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 dark:text-neutral-300 dark:hover:bg-neutral-800 dark:hover:text-neutral-100';

const footerClass = 'flex items-center justify-end gap-2 pt-2';

const bodyClass = 'text-sm leading-relaxed';

export const KobalteModal: Component<KobalteModalProps> = (props) => {
  return (
    <Dialog
      open={props.open}
      onOpenChange={(open) => props.onOpenChange?.(open)}
    >
      <Dialog.Portal>
        <Dialog.Overlay class={overlayClass} />
        <div class={positionerClass}>
          <Dialog.Content
            class={`${contentBaseClass} ${props.contentClass ?? ''}`.trim()}
          >
            <header class={headerClass}>
              <Dialog.Title class={titleClass}>{props.title}</Dialog.Title>
              <Show when={!props.hideCloseButton}>
                <Dialog.CloseButton
                  class={closeButtonClass}
                  aria-label='Close dialog'
                >
                  {/* Visual glyph; aria-label provides the accessible name. */}
                  <span aria-hidden='true'>×</span>
                </Dialog.CloseButton>
              </Show>
            </header>
            <Show when={props.description}>
              <Dialog.Description class={descriptionClass}>
                {props.description}
              </Dialog.Description>
            </Show>
            <div class={bodyClass}>{props.children}</div>
            <Show when={props.footer}>
              <footer class={footerClass}>{props.footer}</footer>
            </Show>
          </Dialog.Content>
        </div>
      </Dialog.Portal>
    </Dialog>
  );
};
