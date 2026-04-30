// Phase 5.1 — FootnoteArea.
//
// Bottom-of-page region listing footnote bodies with their display
// numbers. One <FootnoteArea /> per page; the parent
// (`<FootnotesController />`) selects which footnotes belong on
// each page from `RenderPlan.pages[i].footnotes` and passes them
// to this component.
//
// Pure presentational.

import { type Component, For } from 'solid-js';

export interface FootnoteAreaItem {
  id: string;
  displayNumber: number;
  /** Plain-text rendering of the footnote body. */
  bodyText: string;
}

export interface FootnoteAreaProps {
  items: FootnoteAreaItem[];
  class?: string | undefined;
}

const sectionClass =
  'border-t border-neutral-300 pt-2 mt-4 text-xs text-neutral-700 dark:border-neutral-700 dark:text-neutral-300';

const itemClass = 'flex gap-2 py-0.5';

const numberClass = 'font-medium text-blue-700 dark:text-blue-300';

export const FootnoteArea: Component<FootnoteAreaProps> = (props) => {
  return (
    <section
      class={`${sectionClass} ${props.class ?? ''}`.trim()}
      aria-label='Footnotes'
    >
      <For each={props.items}>
        {(item) => (
          <p class={itemClass} data-footnote-id={item.id}>
            <span class={numberClass}>{item.displayNumber}.</span>
            <span>{item.bodyText}</span>
          </p>
        )}
      </For>
    </section>
  );
};
