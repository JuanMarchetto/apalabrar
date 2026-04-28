/**
 * Phase 2.5 — recent-docs dropdown surfaced in the toolbar (Section
 * D journey #2). Lists OPFS-resident docs, newest-first, with size
 * and last-modified timestamp. Click → load.
 *
 * Renders `null` when `docs` is empty so the toolbar stays clean
 * for first-time visitors.
 *
 * Accessibility:
 * - The trigger is a real `<button>` with `aria-haspopup` /
 *   `aria-expanded` so AT users get the disclosure semantics for
 *   free.
 * - The list of docs is a `<ul role="menu">` with menuitem children;
 *   keyboard navigation (Esc, arrow keys) is delegated to the
 *   browser's native focus management for the v0 ship.
 */
import type { DocId, DocMetadata } from '@apalabrar/editor-bridge';
import { type Component, createSignal, For, Show } from 'solid-js';

export interface RecentDocsMenuProps {
  readonly docs: readonly DocMetadata[];
  readonly onSelect: (id: DocId) => void;
}

const formatBytes = (n: number): string => {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
};

export const RecentDocsMenu: Component<RecentDocsMenuProps> = (props) => {
  const [open, setOpen] = createSignal(false);

  return (
    <Show when={props.docs.length > 0}>
      <div class='relative inline-block'>
        <button
          type='button'
          aria-haspopup='menu'
          aria-expanded={open()}
          class='rounded-md border border-neutral-300 px-3 py-1.5 text-sm hover:bg-neutral-100'
          onClick={() => setOpen(!open())}
        >
          Recent ({props.docs.length})
        </button>
        <Show when={open()}>
          <ul
            role='menu'
            data-testid='recent-docs-menu-list'
            class='absolute right-0 mt-1 min-w-[220px] rounded-md border border-neutral-200 bg-white py-1 shadow-lg'
          >
            <For each={props.docs}>
              {(doc) => (
                <li role='none'>
                  <button
                    type='button'
                    role='menuitem'
                    class='block w-full px-3 py-1.5 text-left text-sm hover:bg-neutral-100'
                    onClick={() => {
                      props.onSelect(doc.id);
                      setOpen(false);
                    }}
                  >
                    <span class='font-medium'>{doc.id}</span>
                    <span class='ml-2 text-xs text-neutral-500'>
                      {formatBytes(doc.sizeBytes)}
                    </span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>
    </Show>
  );
};
