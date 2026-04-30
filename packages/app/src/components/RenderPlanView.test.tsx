// Phase 5.7.3 — RenderPlanView contract.
//
// The painter takes a `RenderPlan` from the wasm layout engine and
// projects it to absolutely-positioned page elements. v0 paints
// block-level text (no per-glyph positioning yet — that lands once
// the input layer in 5.8 needs caret hit-testing). Pages stack
// vertically with a paper-style box-shadow so the reading
// experience matches Word's "page view".

import { render } from '@solidjs/testing-library';
import { describe, expect, it } from 'vitest';

import { type BlockBox, type Page, type RenderPlan, type Viewport } from '@apalabrar/editor-bridge';
import { RenderPlanView } from './RenderPlanView';

const VP: Viewport = {
  pageWidthPx: 816,
  pageHeightPx: 1056,
  marginPx: 96,
};

function paragraphBox(blockIndex: number, originYPx: number): BlockBox {
  return {
    blockIndex,
    kind: { type: 'Paragraph' },
    originXPx: 0,
    originYPx,
    widthPx: 624,
    heightPx: 20,
    lines: [{ widthPx: 200, heightPx: 16, baselineYPx: 12 }],
    lineRange: { start: 0, end: 1 },
  };
}

function page(pageNumber: number, blocks: BlockBox[]): Page {
  return { blocks, pageNumber, footnotes: [] };
}

function plan(pages: Page[]): RenderPlan {
  return { pages, dirtyRects: [], glyphRuns: [], footnoteRefs: [] };
}

describe('RenderPlanView', () => {
  it('renders one page element per page in the plan', () => {
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([page(1, []), page(2, []), page(3, [])])}
        text=''
        viewport={VP}
      />
    ));
    const pages = container.querySelectorAll('[data-page-number]');
    expect(pages).toHaveLength(3);
  });

  it('tags each page with its 1-indexed page number', () => {
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([page(1, []), page(2, [])])}
        text=''
        viewport={VP}
      />
    ));
    const numbers = Array.from(
      container.querySelectorAll('[data-page-number]'),
    ).map((el) => el.getAttribute('data-page-number'));
    expect(numbers).toEqual(['1', '2']);
  });

  it('positions each block absolutely using viewport margin + page-local origin', () => {
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([page(1, [paragraphBox(0, 100)])])}
        text='hello'
        viewport={VP}
      />
    ));
    const block = container.querySelector(
      '[data-block-index="0"]',
    ) as HTMLElement;
    expect(block).not.toBeNull();
    expect(block.style.position).toBe('absolute');
    // 96 (margin) + 100 (originYPx) = 196
    expect(block.style.top).toBe('196px');
    // 96 (margin) + 0 (originXPx) = 96
    expect(block.style.left).toBe('96px');
  });

  it('paints the corresponding text for each block from the supplied text prop', () => {
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([
          page(1, [paragraphBox(0, 0), paragraphBox(1, 30)]),
        ])}
        text={'first\nsecond'}
        viewport={VP}
      />
    ));
    const blocks = container.querySelectorAll('[data-block-index]');
    expect(blocks[0]?.textContent).toBe('first');
    expect(blocks[1]?.textContent).toBe('second');
  });

  it('renders heading blocks with a larger font size than paragraphs', () => {
    const headingBox: BlockBox = {
      blockIndex: 0,
      kind: { type: 'Heading', level: 1 },
      originXPx: 0,
      originYPx: 0,
      widthPx: 624,
      heightPx: 32,
      lines: [{ widthPx: 200, heightPx: 28, baselineYPx: 22 }],
      lineRange: { start: 0, end: 1 },
    };
    const paraBox = paragraphBox(1, 40);
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([page(1, [headingBox, paraBox])])}
        text='Title\nbody'
        viewport={VP}
      />
    ));
    const heading = container.querySelector(
      '[data-block-index="0"]',
    ) as HTMLElement;
    const para = container.querySelector(
      '[data-block-index="1"]',
    ) as HTMLElement;
    const hSize = parseInt(heading.style.fontSize, 10);
    const pSize = parseInt(para.style.fontSize, 10);
    expect(hSize).toBeGreaterThan(pSize);
  });

  it('paints empty string for blocks whose index is past the text projection', () => {
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([page(1, [paragraphBox(0, 0), paragraphBox(99, 30)])])}
        text='only one line'
        viewport={VP}
      />
    ));
    const second = container.querySelector(
      '[data-block-index="99"]',
    ) as HTMLElement;
    expect(second?.textContent).toBe('');
  });

  it('uses the viewport margin to position blocks (not a hardcoded 96)', () => {
    const customVp: Viewport = {
      pageWidthPx: 600,
      pageHeightPx: 800,
      marginPx: 50,
    };
    const { container } = render(() => (
      <RenderPlanView
        plan={plan([page(1, [paragraphBox(0, 10)])])}
        text='x'
        viewport={customVp}
      />
    ));
    const block = container.querySelector(
      '[data-block-index="0"]',
    ) as HTMLElement;
    // 50 (margin) + 10 (originYPx) = 60
    expect(block.style.top).toBe('60px');
    expect(block.style.left).toBe('50px');
  });
});
