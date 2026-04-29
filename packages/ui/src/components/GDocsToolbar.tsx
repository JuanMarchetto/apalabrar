// Main document toolbar — Google Docs minimalist aesthetic.
//
// Fully presentational + controlled: parent owns state and command
// dispatch. We expose:
//   - Undo / Redo
//   - Paragraph style, font family, font size (native <select> for v0)
//   - Bold / Italic / Underline (aria-pressed toggle buttons)
//   - Alignment + list as small toggle groups
//   - Link / Comment quick-insert buttons
//   - WordClassicToggle (the layout-switcher placeholder)
//
// Native <select> instead of kobalte's Select keeps the bundle small
// and has automatic touch / mobile / screen-reader handling. We can
// upgrade to kobalte Select if we need custom rendering of options
// (e.g. previewing each font in its own typeface).

import { type Component, For } from 'solid-js';
import { type WordClassicMode, WordClassicToggle } from './WordClassicToggle';

export type ParagraphStyle =
  | 'normal'
  | 'title'
  | 'heading1'
  | 'heading2'
  | 'heading3'
  | 'heading4'
  | 'heading5'
  | 'heading6';

export type Alignment = 'left' | 'center' | 'right' | 'justify';

export type ListKind = 'none' | 'bullet' | 'numbered';

export interface GDocsToolbarProps {
  canUndo: boolean;
  canRedo: boolean;

  paragraphStyle: ParagraphStyle;
  fontFamily: string;
  fontFamilies: ReadonlyArray<string>;
  fontSize: number;
  fontSizes: ReadonlyArray<number>;

  bold: boolean;
  italic: boolean;
  underline: boolean;

  alignment: Alignment;
  list: ListKind;

  layoutMode: WordClassicMode;

  onUndo?: () => void;
  onRedo?: () => void;
  onParagraphStyleChange?: (style: ParagraphStyle) => void;
  onFontFamilyChange?: (family: string) => void;
  onFontSizeChange?: (size: number) => void;
  onToggleBold?: () => void;
  onToggleItalic?: () => void;
  onToggleUnderline?: () => void;
  onAlignmentChange?: (alignment: Alignment) => void;
  onListChange?: (list: ListKind) => void;
  onLink?: () => void;
  onComment?: () => void;
  onLayoutModeChange?: (mode: WordClassicMode) => void;

  class?: string;
}

const PARAGRAPH_STYLE_LABELS: Record<ParagraphStyle, string> = {
  normal: 'Normal text',
  title: 'Title',
  heading1: 'Heading 1',
  heading2: 'Heading 2',
  heading3: 'Heading 3',
  heading4: 'Heading 4',
  heading5: 'Heading 5',
  heading6: 'Heading 6',
};

const PARAGRAPH_STYLES: ReadonlyArray<ParagraphStyle> = [
  'normal',
  'title',
  'heading1',
  'heading2',
  'heading3',
  'heading4',
  'heading5',
  'heading6',
];

const ALIGNMENT_OPTIONS: ReadonlyArray<{ value: Alignment; label: string; glyph: string; }> = [
  { value: 'left', label: 'Align left', glyph: '⇤' },
  { value: 'center', label: 'Align center', glyph: '↔' },
  { value: 'right', label: 'Align right', glyph: '⇥' },
  { value: 'justify', label: 'Justify', glyph: '☰' },
];

const LIST_OPTIONS: ReadonlyArray<{ value: ListKind; label: string; glyph: string; }> = [
  { value: 'bullet', label: 'Bulleted list', glyph: '•' },
  { value: 'numbered', label: 'Numbered list', glyph: '1.' },
];

const rootClass =
  'flex flex-wrap items-center gap-2 border-b border-neutral-200 bg-white p-2 text-sm dark:border-neutral-800 dark:bg-neutral-900';

const groupClass = 'flex items-center gap-1';

const dividerClass = 'h-6 w-px bg-neutral-300 dark:bg-neutral-700';

const buttonClass =
  'inline-flex h-8 w-8 items-center justify-center rounded text-neutral-700 transition hover:bg-neutral-100 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 dark:text-neutral-200 dark:hover:bg-neutral-800';

const toggleClass =
  `${buttonClass} aria-pressed:bg-blue-100 aria-pressed:text-blue-900 dark:aria-pressed:bg-blue-900/40 dark:aria-pressed:text-blue-100`;

const selectClass =
  'rounded border border-neutral-300 bg-white px-2 py-1 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-neutral-700 dark:bg-neutral-900';

const layoutToggleClass = 'ml-auto';

export const GDocsToolbar: Component<GDocsToolbarProps> = (props) => {
  return (
    <div
      role='toolbar'
      aria-label='Document toolbar'
      class={`${rootClass} ${props.class ?? ''}`.trim()}
    >
      <div class={groupClass}>
        <button
          type='button'
          class={buttonClass}
          aria-label='Undo'
          disabled={!props.canUndo}
          onClick={() => props.onUndo?.()}
        >
          ↶
        </button>
        <button
          type='button'
          class={buttonClass}
          aria-label='Redo'
          disabled={!props.canRedo}
          onClick={() => props.onRedo?.()}
        >
          ↷
        </button>
      </div>

      <div class={dividerClass} aria-hidden='true' />

      <div class={groupClass}>
        {
          /* Each <select> wrapped in its own <span> — Solid + happy-dom
            currently crashes when two <select> are direct siblings
            (https://github.com/solidjs/solid/issues — workaround.
            The wrapper is invisible visually since it's display: contents. */
        }
        <span class='contents'>
          <select
            aria-label='Paragraph style'
            class={selectClass}
            value={props.paragraphStyle}
            onChange={(event) =>
              props.onParagraphStyleChange?.(
                event.currentTarget.value as ParagraphStyle,
              )}
          >
            <For each={PARAGRAPH_STYLES}>
              {(style) => <option value={style}>{PARAGRAPH_STYLE_LABELS[style]}</option>}
            </For>
          </select>
        </span>

        <span class='contents'>
          <select
            aria-label='Font family'
            class={selectClass}
            value={props.fontFamily}
            onChange={(event) => props.onFontFamilyChange?.(event.currentTarget.value)}
          >
            <For each={props.fontFamilies}>
              {(family) => <option value={family}>{family}</option>}
            </For>
          </select>
        </span>

        <span class='contents'>
          <select
            aria-label='Font size'
            class={selectClass}
            value={props.fontSize}
            onChange={(event) => props.onFontSizeChange?.(Number(event.currentTarget.value))}
          >
            <For each={props.fontSizes}>
              {(size) => <option value={size}>{size}</option>}
            </For>
          </select>
        </span>
      </div>

      <div class={dividerClass} aria-hidden='true' />

      <div class={groupClass}>
        <button
          type='button'
          class={toggleClass}
          aria-label='Bold'
          aria-pressed={props.bold}
          onClick={() => props.onToggleBold?.()}
        >
          <strong>B</strong>
        </button>
        <button
          type='button'
          class={toggleClass}
          aria-label='Italic'
          aria-pressed={props.italic}
          onClick={() => props.onToggleItalic?.()}
        >
          <em>I</em>
        </button>
        <button
          type='button'
          class={toggleClass}
          aria-label='Underline'
          aria-pressed={props.underline}
          onClick={() => props.onToggleUnderline?.()}
        >
          <span class='underline'>U</span>
        </button>
      </div>

      <div class={dividerClass} aria-hidden='true' />

      <div class={groupClass} role='group' aria-label='Text alignment'>
        <For each={ALIGNMENT_OPTIONS}>
          {(option) => (
            <button
              type='button'
              class={toggleClass}
              aria-label={option.label}
              aria-pressed={props.alignment === option.value}
              onClick={() => props.onAlignmentChange?.(option.value)}
            >
              {option.glyph}
            </button>
          )}
        </For>
      </div>

      <div class={dividerClass} aria-hidden='true' />

      <div class={groupClass} role='group' aria-label='Lists'>
        <For each={LIST_OPTIONS}>
          {(option) => (
            <button
              type='button'
              class={toggleClass}
              aria-label={option.label}
              aria-pressed={props.list === option.value}
              onClick={() =>
                props.onListChange?.(
                  props.list === option.value ? 'none' : option.value,
                )}
            >
              {option.glyph}
            </button>
          )}
        </For>
      </div>

      <div class={dividerClass} aria-hidden='true' />

      <div class={groupClass}>
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

      <WordClassicToggle
        class={layoutToggleClass}
        value={props.layoutMode}
        onChange={(mode) => props.onLayoutModeChange?.(mode)}
      />
    </div>
  );
};
