//! Property-based tests for the layout engine.
//!
//! Invariants codified here (per `tdd-execution-plan.md` rule 5: property-based
//! tests on layout):
//!
//! 1. `block_box_count(layout(doc)) == doc.len()` for any non-empty doc.
//! 2. `layout(doc) == layout(doc)` (determinism).
//! 3. Every `block_index` in the result is unique and falls in `0..doc.len()`.
//! 4. Every `BlockBox.height_px ≤ Viewport.content_height()` (no block is
//!    bigger than a single page in this gate; if a paragraph would exceed a
//!    page it's still legal — but for the synthetic short-block corpus the
//!    invariant holds and a violation flags a bug).
//! 5. `relayout_after_change(doc, i)` for any valid `i` returns the same
//!    result as `layout(doc)` (incremental ≡ fresh).
//! 6. `relayout_after_change(doc, i)` for any `i ≥ doc.len()` returns
//!    `InvalidBlockIndex`.

use apalabrar_layout::{Block, Document, Engine, Error, LETTER_AT_96DPI};
use proptest::prelude::*;

fn engine() -> Engine {
    Engine::new(LETTER_AT_96DPI).expect("font loads")
}

/// Strategy: a small corpus of short-text blocks. Strings are constrained to
/// printable ASCII to avoid pulling in font fallback paths that aren't covered
/// by the bundled DejaVu Sans face.
fn block_strategy() -> impl Strategy<Value = Block> {
    prop_oneof![
        "[a-zA-Z0-9 ,.;:?!]{1,40}".prop_map(|s| Block::Paragraph { text: s }),
        (1u8..=6, "[a-zA-Z0-9 ,.;:?!]{1,40}")
            .prop_map(|(level, text)| Block::Heading { level, text }),
        (0u8..=4, "[a-zA-Z0-9 ,.;:?!]{1,40}")
            .prop_map(|(indent, text)| Block::ListItem { indent, text }),
    ]
}

fn document_strategy(min: usize, max: usize) -> impl Strategy<Value = Document> {
    prop::collection::vec(block_strategy(), min..=max).prop_map(Document::new)
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        max_shrink_iters: 64,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_block_box_count_equals_doc_len(doc in document_strategy(1, 30)) {
        let mut e = engine();
        let result = e.layout(&doc);
        prop_assert_eq!(result.block_box_count(), doc.len());
    }

    #[test]
    fn prop_layout_is_deterministic(doc in document_strategy(1, 20)) {
        let mut e1 = engine();
        let mut e2 = engine();
        prop_assert_eq!(e1.layout(&doc), e2.layout(&doc));
    }

    #[test]
    fn prop_each_block_index_appears_exactly_once(doc in document_strategy(1, 50)) {
        let mut e = engine();
        let result = e.layout(&doc);
        let mut indices: Vec<usize> = result
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(|b| b.block_index))
            .collect();
        indices.sort_unstable();
        let expected: Vec<usize> = (0..doc.len()).collect();
        prop_assert_eq!(indices, expected);
    }

    #[test]
    fn prop_no_block_box_taller_than_a_page(doc in document_strategy(1, 30)) {
        let mut e = engine();
        let result = e.layout(&doc);
        let max = LETTER_AT_96DPI.content_height();
        for page in &result.pages {
            for bx in &page.blocks {
                prop_assert!(
                    bx.height_px <= max + 0.1,
                    "block {} height {} exceeded page content height {}",
                    bx.block_index, bx.height_px, max,
                );
            }
        }
    }

    #[test]
    fn prop_relayout_oob_always_errors(
        doc in document_strategy(1, 10),
        bump in 0usize..50,
    ) {
        let mut e = engine();
        let _ = e.layout(&doc);
        let oob = doc.len() + bump;
        let result = e.relayout_after_change(&doc, oob);
        let is_oob_error = matches!(result, Err(Error::InvalidBlockIndex { .. }));
        prop_assert!(is_oob_error, "relayout at oob index must error");
    }

    #[test]
    fn prop_relayout_in_place_matches_fresh_layout(
        doc in document_strategy(2, 15),
        idx_seed in 0usize..15,
    ) {
        let idx = idx_seed % doc.len();
        let mut incremental_engine = engine();
        let _ = incremental_engine.layout(&doc);
        let incremental = incremental_engine
            .relayout_after_change(&doc, idx)
            .expect("valid index");

        let mut fresh_engine = engine();
        let fresh = fresh_engine.layout(&doc);

        prop_assert_eq!(incremental, fresh);
    }
}
