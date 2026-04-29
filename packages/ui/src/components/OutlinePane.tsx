// Document outline navigation. Receives a flat list of headings already
// projected from the doc model (`{blockId, level, text}`); emits `onJump`
// when the user clicks one. Headings indent by level so a glance reveals
// the structure.
//
// We don't recurse into a nested tree because:
//   - The doc-model exposes a flat block list; flat-with-indent is faster
//     to render and maintain than a tree projection.
//   - Screen readers traverse a flat list-of-buttons just as well; the
//     `aria-level` on each item conveys hierarchy without nesting.

import { type Component, For, Show } from 'solid-js';

export type OutlineHeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

export interface OutlineHeading {
  /** doc-model BlockId — opaque to this component, passed back via onJump. */
  blockId: string;
  level: OutlineHeadingLevel;
  text: string;
}

export interface OutlinePaneProps {
  headings: ReadonlyArray<OutlineHeading>;
  /** Currently focused heading; gets aria-current="location" + active style. */
  activeBlockId?: string;
  /** Fired with the clicked heading's blockId. */
  onJump?: (blockId: string) => void;
  /** Override the empty-state copy (defaults to "No headings yet."). */
  emptyMessage?: string;
  /** Extra root class for layout overrides. */
  class?: string;
}

const rootClass = 'flex h-full w-full flex-col gap-1 overflow-y-auto p-2 text-sm';

const listClass = 'flex flex-col gap-0.5';

const buttonBaseClass =
  'block w-full truncate rounded px-2 py-1 text-left text-neutral-700 transition hover:bg-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 dark:text-neutral-300 dark:hover:bg-neutral-800';

const buttonActiveClass =
  'bg-blue-50 font-medium text-blue-900 dark:bg-blue-900/40 dark:text-blue-100';

const emptyClass = 'px-2 py-4 text-neutral-600 dark:text-neutral-300';

const LEVEL_INDENT: Record<OutlineHeadingLevel, string> = {
  1: 'pl-2',
  2: 'pl-4',
  3: 'pl-6',
  4: 'pl-8',
  5: 'pl-10',
  6: 'pl-12',
};

export const OutlinePane: Component<OutlinePaneProps> = (props) => {
  const isActive = (id: string) => props.activeBlockId === id;

  return (
    <nav
      aria-label='Document outline'
      class={`${rootClass} ${props.class ?? ''}`.trim()}
    >
      <Show
        when={props.headings.length > 0}
        fallback={
          <p class={emptyClass}>
            {props.emptyMessage ?? 'No headings yet.'}
          </p>
        }
      >
        <ul class={listClass} role='list'>
          <For each={props.headings}>
            {(heading) => {
              const indent = LEVEL_INDENT[heading.level];
              return (
                <li>
                  <button
                    type='button'
                    class={`${buttonBaseClass} ${indent} ${
                      isActive(heading.blockId) ? buttonActiveClass : ''
                    }`.trim()}
                    data-level={heading.level}
                    aria-current={isActive(heading.blockId) ? 'location' : undefined}
                    onClick={() => props.onJump?.(heading.blockId)}
                  >
                    {heading.text}
                  </button>
                </li>
              );
            }}
          </For>
        </ul>
      </Show>
    </nav>
  );
};
