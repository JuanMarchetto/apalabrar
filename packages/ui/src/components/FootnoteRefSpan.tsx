// Phase 5.1 — FootnoteRefSpan.
//
// Inline superscript anchor rendered on top of the `\u{E001}`
// footnote marker codepoint. Carries the display number (1-indexed
// position-order) and the footnote id so the editor can wire a
// click to the corresponding entry in the page's footnote area.
//
// Pure presentational; click callback is wired by the parent
// (`<FootnotesController />`).

import { type Component } from 'solid-js';

export interface FootnoteRefSpanProps {
  /** Footnote id. Used for the data attribute + click payload. */
  id: string;
  /** 1-indexed display number painted as superscript. */
  displayNumber: number;
  onClick?: ((id: string) => void) | undefined;
  class?: string | undefined;
}

const supClass =
  'cursor-pointer text-[0.75em] align-super font-medium text-blue-700 hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 dark:text-blue-300';

export const FootnoteRefSpan: Component<FootnoteRefSpanProps> = (props) => {
  return (
    <sup
      class={`${supClass} ${props.class ?? ''}`.trim()}
      data-footnote-id={props.id}
      aria-label={`Footnote ${props.displayNumber}`}
      role='button'
      tabindex={0}
      onClick={() => props.onClick?.(props.id)}
    >
      {props.displayNumber}
    </sup>
  );
};
