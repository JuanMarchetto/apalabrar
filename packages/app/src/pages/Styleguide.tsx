// Styleguide route — renders all @apalabrar/ui components against
// sample data, with a theme switcher (light / dark / high-contrast).
//
// Used by Playwright visual-regression specs; each component is wrapped
// in a section with a stable id so the spec can target it precisely.

import {
  CommentsSidebar,
  type CommentThread,
  FindReplaceBar,
  FloatingSelectionToolbar,
  GDocsToolbar,
  type GDocsToolbarProps,
  KobalteModal,
  type OutlineHeading,
  OutlinePane,
  type WordClassicMode,
  WordClassicToggle,
} from '@apalabrar/ui';
import { type Component, createSignal, For, type JSX } from 'solid-js';

type Theme = 'light' | 'dark' | 'high-contrast';

const THEMES: ReadonlyArray<{ value: Theme; label: string; }> = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
  { value: 'high-contrast', label: 'High contrast' },
];

const sampleHeadings: ReadonlyArray<OutlineHeading> = [
  { blockId: 'h1-intro', level: 1, text: 'Introduction' },
  { blockId: 'h2-bg', level: 2, text: 'Background' },
  { blockId: 'h2-method', level: 2, text: 'Method' },
  { blockId: 'h3-sample', level: 3, text: 'Sample selection' },
  { blockId: 'h2-results', level: 2, text: 'Results' },
  { blockId: 'h2-disc', level: 2, text: 'Discussion' },
];

const sampleThreads: ReadonlyArray<CommentThread> = [
  {
    id: 'c1',
    author: 'Alice',
    body: 'This sentence is unclear — could you rephrase?',
    createdAt: '2026-04-29T10:00:00Z',
    status: 'open',
    anchorBlockId: 'h2-bg',
    replies: [
      {
        id: 'c1r1',
        author: 'Bob',
        body: 'Agreed, will rephrase in v2.',
        createdAt: '2026-04-29T10:30:00Z',
      },
    ],
  },
  {
    id: 'c2',
    author: 'Carol',
    body: 'Citation needed for the 1928 figure.',
    createdAt: '2026-04-28T16:45:00Z',
    status: 'resolved',
    anchorBlockId: 'h2-method',
  },
];

const fontFamilies = [
  'Inter',
  'Source Serif Pro',
  'JetBrains Mono',
  'Crimson Pro',
];

const fontSizes = [10, 11, 12, 14, 16, 18, 24, 32];

const Section: Component<{
  id: string;
  title: string;
  children: JSX.Element;
}> = (props) => {
  return (
    <section
      id={props.id}
      data-styleguide-section={props.id}
      class='border-b border-neutral-200 px-6 py-8 dark:border-neutral-800'
    >
      <h2 class='mb-4 text-xs font-semibold uppercase tracking-wide text-neutral-700 dark:text-neutral-300'>
        {props.title}
      </h2>
      {props.children}
    </section>
  );
};

export const Styleguide: Component = () => {
  const [theme, setTheme] = createSignal<Theme>('light');
  const [layoutMode, setLayoutMode] = createSignal<WordClassicMode>('modern');
  const [bold, setBold] = createSignal(false);
  const [italic, setItalic] = createSignal(true);
  const [underline, setUnderline] = createSignal(false);
  const [alignment, setAlignment] = createSignal<
    GDocsToolbarProps['alignment']
  >('left');
  const [list, setList] = createSignal<GDocsToolbarProps['list']>('none');
  const [paragraphStyle, setParagraphStyle] = createSignal<
    GDocsToolbarProps['paragraphStyle']
  >('normal');
  const [fontFamily, setFontFamily] = createSignal('Inter');
  const [fontSize, setFontSize] = createSignal(12);

  const [findQuery, setFindQuery] = createSignal('lorem');
  const [replaceQuery, setReplaceQuery] = createSignal('ipsum');
  const [caseSensitive, setCaseSensitive] = createSignal(false);
  const [wholeWord, setWholeWord] = createSignal(true);

  const [modalOpen, setModalOpen] = createSignal(false);

  const isDark = () => theme() === 'dark' || theme() === 'high-contrast';
  return (
    <div
      data-theme={theme()}
      classList={{
        'min-h-screen bg-white text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100': true,
        dark: isDark(),
      }}
    >
      <header class='flex items-center justify-between border-b border-neutral-200 px-6 py-4 dark:border-neutral-800'>
        <h1 class='text-lg font-semibold'>Apalabrar Styleguide</h1>
        <div class='flex items-center gap-2'>
          <label for='theme-select' class='text-sm'>
            Theme:
          </label>
          <select
            id='theme-select'
            data-testid='theme-select'
            class='rounded border border-neutral-300 px-2 py-1 text-sm dark:border-neutral-700'
            value={theme()}
            onChange={(event) => setTheme(event.currentTarget.value as Theme)}
          >
            <For each={THEMES}>
              {(t) => <option value={t.value}>{t.label}</option>}
            </For>
          </select>
        </div>
      </header>

      <Section id='gdocs-toolbar' title='GDocsToolbar'>
        <GDocsToolbar
          canUndo={true}
          canRedo={false}
          paragraphStyle={paragraphStyle()}
          fontFamily={fontFamily()}
          fontFamilies={fontFamilies}
          fontSize={fontSize()}
          fontSizes={fontSizes}
          bold={bold()}
          italic={italic()}
          underline={underline()}
          alignment={alignment()}
          list={list()}
          layoutMode={layoutMode()}
          onParagraphStyleChange={setParagraphStyle}
          onFontFamilyChange={setFontFamily}
          onFontSizeChange={setFontSize}
          onToggleBold={() => setBold(!bold())}
          onToggleItalic={() => setItalic(!italic())}
          onToggleUnderline={() => setUnderline(!underline())}
          onAlignmentChange={setAlignment}
          onListChange={setList}
          onLayoutModeChange={setLayoutMode}
        />
      </Section>

      <Section id='word-classic-toggle' title='WordClassicToggle'>
        <WordClassicToggle value={layoutMode()} onChange={setLayoutMode} />
      </Section>

      <Section id='outline-pane' title='OutlinePane'>
        <div class='h-64 max-w-xs border border-neutral-200 dark:border-neutral-800'>
          <OutlinePane
            headings={sampleHeadings}
            activeBlockId='h2-method'
          />
        </div>
      </Section>

      <Section id='comments-sidebar' title='CommentsSidebar'>
        <div class='h-96 max-w-md border border-neutral-200 dark:border-neutral-800'>
          <CommentsSidebar
            threads={sampleThreads}
            activeThreadId='c1'
            onReply={() => {}}
          />
        </div>
      </Section>

      <Section id='find-replace-bar' title='FindReplaceBar'>
        <FindReplaceBar
          findQuery={findQuery()}
          replaceQuery={replaceQuery()}
          matchCount={12}
          currentMatch={2}
          caseSensitive={caseSensitive()}
          wholeWord={wholeWord()}
          onFindQueryChange={setFindQuery}
          onReplaceQueryChange={setReplaceQuery}
          onCaseSensitiveChange={setCaseSensitive}
          onWholeWordChange={setWholeWord}
          onClose={() => {}}
        />
      </Section>

      <Section id='floating-selection-toolbar' title='FloatingSelectionToolbar'>
        <div class='relative h-24 rounded border border-dashed border-neutral-300 dark:border-neutral-700'>
          <FloatingSelectionToolbar
            visible={true}
            position={{ x: 24, y: 24 }}
            bold={true}
            italic={false}
            underline={false}
          />
        </div>
      </Section>

      <Section id='kobalte-modal' title='KobalteModal'>
        <button
          type='button'
          data-testid='open-modal'
          class='rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700'
          onClick={() => setModalOpen(true)}
        >
          Open modal
        </button>
        <KobalteModal
          open={modalOpen()}
          onOpenChange={setModalOpen}
          title='Insert citation'
          description='Pick a source from your reference library.'
          footer={
            <>
              <button
                type='button'
                class='rounded border border-neutral-300 px-3 py-1 text-sm dark:border-neutral-700'
                onClick={() => setModalOpen(false)}
              >
                Cancel
              </button>
              <button
                type='button'
                class='rounded bg-blue-600 px-3 py-1 text-sm font-medium text-white'
                onClick={() => setModalOpen(false)}
              >
                Insert
              </button>
            </>
          }
        >
          <p>This is a sample modal for visual regression and a11y testing.</p>
        </KobalteModal>
      </Section>
    </div>
  );
};
