// Toggle between the Modern (Google Docs minimalist) and Classic
// (Word ribbon) toolbar layouts.
//
// v0 ships only the Modern layout — `classic` is a forward-compatible
// signal so consumers can persist the user's preference now and the
// ribbon implementation can land in v1 without a migration. The toggle
// itself is fully functional today.
//
// We use kobalte's SegmentedControl (built on RadioGroup) for proper
// radiogroup semantics + arrow-key navigation out of the box.

import { SegmentedControl } from '@kobalte/core/segmented-control';
import { type Component, For } from 'solid-js';

export type WordClassicMode = 'modern' | 'classic';

export interface WordClassicToggleProps {
  /** Currently selected mode. */
  value: WordClassicMode;
  /** Fired when the user picks a different mode. */
  onChange?: (value: WordClassicMode) => void;
  /** Extra root class for layout overrides. */
  class?: string;
}

interface Option {
  value: WordClassicMode;
  label: string;
}

const OPTIONS: ReadonlyArray<Option> = [
  { value: 'modern', label: 'Modern' },
  { value: 'classic', label: 'Classic' },
];

const rootClass =
  'inline-flex items-center gap-1 rounded-md bg-neutral-100 p-1 text-sm dark:bg-neutral-800';

const itemClass =
  'relative flex cursor-pointer items-center rounded px-3 py-1 transition data-[checked]:bg-white data-[checked]:text-neutral-900 data-[checked]:shadow-sm dark:data-[checked]:bg-neutral-700 dark:data-[checked]:text-neutral-100';

const labelClass = 'select-none';

export const WordClassicToggle: Component<WordClassicToggleProps> = (props) => {
  return (
    <SegmentedControl
      value={props.value}
      onChange={(v) => props.onChange?.(v as WordClassicMode)}
      aria-label='Toolbar layout'
      class={`${rootClass} ${props.class ?? ''}`.trim()}
    >
      <For each={OPTIONS}>
        {(option) => (
          <SegmentedControl.Item value={option.value} class={itemClass}>
            <SegmentedControl.ItemInput />
            <SegmentedControl.ItemControl>
              <SegmentedControl.ItemLabel class={labelClass}>
                {option.label}
              </SegmentedControl.ItemLabel>
            </SegmentedControl.ItemControl>
          </SegmentedControl.Item>
        )}
      </For>
    </SegmentedControl>
  );
};
