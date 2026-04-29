// Bubble formatting toolbar that floats above an active text selection.
//
// Positioning is the consumer's job: they pass {x, y} in viewport pixels
// (typically the top-center of the selection rect, minus the toolbar's
// height). We render with `position: absolute` so the consumer can place
// us inside their own positioning container.
//
// Visibility is also caller-controlled — when no text is selected, the
// parent passes `visible={false}` and we render nothing. We don't track
// document.getSelection() ourselves because the editor already maintains
// canonical selection state in the doc-model; reading the DOM Selection
// would race with composition events.

import { type Component, Show } from 'solid-js';

export interface FloatingSelectionToolbarProps {
  visible: boolean;
  /** Position in viewport pixels (consumer is responsible for math). */
  position: { x: number; y: number; };
  bold: boolean;
  italic: boolean;
  underline: boolean;

  onToggleBold?: () => void;
  onToggleItalic?: () => void;
  onToggleUnderline?: () => void;
  onLink?: () => void;
  onComment?: () => void;

  class?: string;
}

const rootClass =
  'absolute z-30 inline-flex items-center gap-1 rounded-md border border-neutral-200 bg-white p-1 shadow-md dark:border-neutral-700 dark:bg-neutral-900';

const buttonClass =
  'inline-flex h-7 w-7 items-center justify-center rounded text-sm font-medium text-neutral-700 hover:bg-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 aria-pressed:bg-blue-100 aria-pressed:text-blue-900 dark:text-neutral-200 dark:hover:bg-neutral-800 dark:aria-pressed:bg-blue-900/40 dark:aria-pressed:text-blue-100';

export const FloatingSelectionToolbar: Component<
  FloatingSelectionToolbarProps
> = (props) => {
  return (
    <Show when={props.visible}>
      <div
        role='toolbar'
        aria-label='Selection formatting'
        class={`${rootClass} ${props.class ?? ''}`.trim()}
        style={{
          left: `${props.position.x}px`,
          top: `${props.position.y}px`,
        }}
      >
        <button
          type='button'
          class={buttonClass}
          aria-label='Bold'
          aria-pressed={props.bold}
          onClick={() => props.onToggleBold?.()}
        >
          <strong>B</strong>
        </button>
        <button
          type='button'
          class={buttonClass}
          aria-label='Italic'
          aria-pressed={props.italic}
          onClick={() => props.onToggleItalic?.()}
        >
          <em>I</em>
        </button>
        <button
          type='button'
          class={buttonClass}
          aria-label='Underline'
          aria-pressed={props.underline}
          onClick={() => props.onToggleUnderline?.()}
        >
          <span class='underline'>U</span>
        </button>
        <button
          type='button'
          class={buttonClass}
          aria-label='Insert link'
          onClick={() => props.onLink?.()}
        >
          🔗
        </button>
        <button
          type='button'
          class={buttonClass}
          aria-label='Add comment'
          onClick={() => props.onComment?.()}
        >
          💬
        </button>
      </div>
    </Show>
  );
};
