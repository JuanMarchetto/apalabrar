// Phase 5.7.3 — RenderPlanView.
//
// Paints a `RenderPlan` (from `core.layout(id, viewport)`) to the
// DOM as a vertical stack of pages with absolutely-positioned
// block boxes. v0 emits one block element per `BlockBox` and
// places the block's text inside it; per-glyph positioning lands
// in the input layer (Phase 5.8) when caret hit-testing demands
// it. The painter is intentionally a pure function of
// `(plan, text, viewport)` so re-runs after an edit (or a window
// resize that changed the viewport) require no state diff — Solid
// reconciles the children via `<For>`.
//
// Why block-level text rather than glyph-level: at the resolution
// the v0 doc-model gives us (text-only, no per-character marks),
// re-shaping in the browser via CSS produces fidelity-equivalent
// output to painting the engine's glyph runs directly. The wasm
// output stays available for hit-testing in 5.8 — we don't drop
// it.

import type { BlockBox, RenderPlan, Viewport } from '@apalabrar/editor-bridge';
import { type Component, For } from 'solid-js';

export interface RenderPlanViewProps {
  /** The latest layout output. Re-rendering with a fresh `plan`
   *  is the supported way to repaint after an edit. */
  readonly plan: RenderPlan;
  /** The doc's plain-text projection. Block N's text is the Nth
   *  line of `text` after splitting on `\n`. */
  readonly text: string;
  /** Page geometry — must match the viewport that produced
   *  `plan`. Used to translate page-local block origins into
   *  CSS absolute coordinates inside each page. */
  readonly viewport: Viewport;
}

/** v0 heading sizes (px) per level 1..6. Matches the canonical
 *  Word default scale closely enough for the paginated view; the
 *  live shaping engine in Rust uses its own metrics so the visual
 *  fidelity will ever-so-slightly drift here from the actual
 *  glyph baseline. Acceptable for v0 — full WYSIWYG glyph paint
 *  arrives with the input layer in 5.8. */
const HEADING_PX_BY_LEVEL: readonly number[] = [28, 24, 20, 18, 16, 15];

const PARAGRAPH_PX = 16;

function fontSizeFor(block: BlockBox): number {
  if (block.kind.type === 'Heading') {
    return HEADING_PX_BY_LEVEL[block.kind.level - 1] ?? PARAGRAPH_PX;
  }
  return PARAGRAPH_PX;
}

export const RenderPlanView: Component<RenderPlanViewProps> = (props) => {
  // The doc-text projection joins blocks with `\n`. Block N's text
  // is the Nth element after splitting; out-of-range indices map
  // to the empty string (the painter trusts the layout's
  // `blockIndex` but tolerates a stale `text` prop arriving out of
  // step with the `plan`).
  const blockTexts = (): string[] => props.text.split('\n');

  return (
    <div class='apalabrar-render-plan' data-testid='render-plan'>
      <For each={props.plan.pages}>
        {(page) => (
          <div
            class='apalabrar-page'
            data-page-number={page.pageNumber}
            style={{
              position: 'relative',
              width: `${props.viewport.pageWidthPx}px`,
              height: `${props.viewport.pageHeightPx}px`,
              background: 'white',
              'box-shadow': '0 2px 8px rgba(0, 0, 0, 0.08)',
              margin: '24px auto',
              'font-family': '"DejaVu Sans", "Helvetica Neue", sans-serif',
            }}
          >
            <For each={page.blocks}>
              {(block) => (
                <div
                  class='apalabrar-block'
                  data-block-index={block.blockIndex}
                  style={{
                    position: 'absolute',
                    left: `${props.viewport.marginPx + block.originXPx}px`,
                    top: `${props.viewport.marginPx + block.originYPx}px`,
                    width: `${block.widthPx}px`,
                    'font-size': `${fontSizeFor(block)}px`,
                    'font-weight': block.kind.type === 'Heading' ? 'bold' : 'normal',
                    'line-height': '1.4',
                    'white-space': 'pre-wrap',
                    'word-break': 'break-word',
                  }}
                >
                  {blockTexts()[block.blockIndex] ?? ''}
                </div>
              )}
            </For>
          </div>
        )}
      </For>
    </div>
  );
};
