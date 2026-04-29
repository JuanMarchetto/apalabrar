// Find & replace bar. Fully controlled: parent owns state, this component
// renders inputs + buttons + counter and emits callbacks.
//
// We keep regex-mode out of v0 (per Phase 4.5 plan). Case-sensitive and
// whole-word are exposed as aria-pressed toggle buttons — same pattern
// VS Code uses; minimal bundle cost vs a full kobalte ToggleButton.
//
// The match counter uses aria-live="polite" so screen reader users hear
// "3 of 12" updates as they iterate.

import { type Component, Show } from 'solid-js';

export interface FindReplaceBarProps {
  /** Current find query. */
  findQuery: string;
  /** Current replace text. */
  replaceQuery: string;
  /** Current match index (0-based). Undefined when no match focused. */
  currentMatch?: number;
  /** Total match count for findQuery. */
  matchCount: number;
  caseSensitive: boolean;
  wholeWord: boolean;

  onFindQueryChange?: (query: string) => void;
  onReplaceQueryChange?: (query: string) => void;
  onCaseSensitiveChange?: (caseSensitive: boolean) => void;
  onWholeWordChange?: (wholeWord: boolean) => void;
  onNext?: () => void;
  onPrev?: () => void;
  onReplace?: () => void;
  onReplaceAll?: () => void;
  onClose?: () => void;

  class?: string;
}

const rootClass =
  'flex flex-wrap items-center gap-2 rounded border border-neutral-300 bg-white p-2 text-sm shadow-sm dark:border-neutral-700 dark:bg-neutral-900';

const inputClass =
  'min-w-40 flex-1 rounded border border-neutral-300 px-2 py-1 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-neutral-700 dark:bg-neutral-800';

const counterClass =
  'min-w-24 text-center text-xs tabular-nums text-neutral-700 dark:text-neutral-300';

const buttonClass =
  'inline-flex items-center justify-center rounded px-2 py-1 text-xs font-medium text-neutral-700 hover:bg-neutral-100 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 dark:text-neutral-200 dark:hover:bg-neutral-800';

const toggleClass =
  'inline-flex items-center justify-center rounded px-2 py-1 text-xs font-medium transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 aria-pressed:bg-blue-100 aria-pressed:text-blue-900 dark:aria-pressed:bg-blue-900/40 dark:aria-pressed:text-blue-100';

const closeClass = `${buttonClass} ml-auto`;

const formatCounter = (matchCount: number, currentMatch?: number): string => {
  if (matchCount === 0) return '0 of 0';
  // currentMatch is 0-indexed internally; display as 1-indexed.
  const current = typeof currentMatch === 'number' ? currentMatch + 1 : 1;
  return `${current} of ${matchCount}`;
};

export const FindReplaceBar: Component<FindReplaceBarProps> = (props) => {
  const noMatches = () => props.matchCount === 0;

  return (
    <form
      role='search'
      aria-label='Find and replace'
      class={`${rootClass} ${props.class ?? ''}`.trim()}
      onSubmit={(event) => {
        event.preventDefault();
        props.onNext?.();
      }}
    >
      <input
        type='search'
        aria-label='Find'
        placeholder='Find'
        class={inputClass}
        value={props.findQuery}
        onInput={(event) => props.onFindQueryChange?.(event.currentTarget.value)}
      />
      <input
        type='text'
        aria-label='Replace'
        placeholder='Replace'
        class={inputClass}
        value={props.replaceQuery}
        onInput={(event) => props.onReplaceQueryChange?.(event.currentTarget.value)}
      />
      <output class={counterClass} aria-live='polite' aria-atomic='true'>
        {formatCounter(props.matchCount, props.currentMatch)}
      </output>
      <button
        type='button'
        class={buttonClass}
        aria-label='Previous match'
        disabled={noMatches()}
        onClick={() => props.onPrev?.()}
      >
        ↑
      </button>
      <button
        type='button'
        class={buttonClass}
        aria-label='Next match'
        disabled={noMatches()}
        onClick={() => props.onNext?.()}
      >
        ↓
      </button>
      <button
        type='button'
        class={toggleClass}
        aria-label='Match case'
        aria-pressed={props.caseSensitive}
        onClick={() => props.onCaseSensitiveChange?.(!props.caseSensitive)}
      >
        Aa
      </button>
      <button
        type='button'
        class={toggleClass}
        aria-label='Whole word'
        aria-pressed={props.wholeWord}
        onClick={() => props.onWholeWordChange?.(!props.wholeWord)}
      >
        ab
      </button>
      <button
        type='button'
        class={buttonClass}
        disabled={noMatches()}
        onClick={() => props.onReplace?.()}
      >
        Replace
      </button>
      <button
        type='button'
        class={buttonClass}
        disabled={noMatches()}
        onClick={() => props.onReplaceAll?.()}
      >
        Replace all
      </button>
      <Show when={props.onClose}>
        <button
          type='button'
          class={closeClass}
          aria-label='Close find and replace'
          onClick={() => props.onClose?.()}
        >
          ✕
        </button>
      </Show>
    </form>
  );
};
