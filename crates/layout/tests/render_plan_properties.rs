//! Phase 4.1 RED — property tests for `apalabrar_layout::layout()`.
//!
//! The page-count invariants come from blueprint Section E (R3) and the
//! TDD plan Section 4c. They are the single source of truth for any
//! incremental relayout strategy: a relayout that violates one of these
//! is broken regardless of micro-benchmarks.

use apalabrar_doc_model::Doc;
use apalabrar_layout::{BlockKind, LETTER_AT_96DPI, layout};
use proptest::prelude::*;

/// A "simple" doc shape: 1..=20 short ASCII paragraphs.
fn arb_paragraphs() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        // 1..=80 chars, ASCII letters + space (no `\n` so each entry
        // stays one block).
        prop::string::string_regex("[a-z ]{1,80}").unwrap(),
        1..=20,
    )
}

fn doc_from(paragraphs: &[String]) -> Doc {
    let mut d = Doc::new();
    d.insert(0, &paragraphs.join("\n"));
    d
}

proptest! {
    /// Determinism: same `Doc` + same `Viewport` produces identical
    /// `RenderPlan`s. Without this, every higher-level invariant is
    /// meaningless (we'd be measuring noise).
    #[test]
    fn layout_is_pure(paragraphs in arb_paragraphs()) {
        let d = doc_from(&paragraphs);
        let p1 = layout(&d, &LETTER_AT_96DPI).unwrap();
        let p2 = layout(&d, &LETTER_AT_96DPI).unwrap();
        prop_assert_eq!(p1, p2);
    }

    /// Conservation: every block in the doc maps to exactly one
    /// `BlockBox` in the plan. Layout cannot drop or duplicate blocks.
    #[test]
    fn block_count_is_conserved(paragraphs in arb_paragraphs()) {
        let d = doc_from(&paragraphs);
        let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
        prop_assert_eq!(plan.block_box_count(), d.block_count());
    }

    /// Block indices are a permutation of `0..doc.block_count()`. Layout
    /// must not invent indices, repeat them, or skip them.
    #[test]
    fn block_indices_are_a_permutation(paragraphs in arb_paragraphs()) {
        let d = doc_from(&paragraphs);
        let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
        let mut indices: Vec<usize> = plan
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(|b| b.block_index))
            .collect();
        indices.sort_unstable();
        let expected: Vec<usize> = (0..d.block_count()).collect();
        prop_assert_eq!(indices, expected);
    }

    /// Page-count invariant R3.a (monotonicity): adding a block never
    /// decreases the page count. Two docs built from the same paragraph
    /// list, the second with one extra paragraph appended.
    #[test]
    fn page_count_does_not_decrease_when_adding_a_block(paragraphs in arb_paragraphs()) {
        let small = doc_from(&paragraphs);
        let mut extended = paragraphs.clone();
        extended.push("appended line".to_owned());
        let big = doc_from(&extended);
        let p_before = layout(&small, &LETTER_AT_96DPI).unwrap().page_count();
        let p_after = layout(&big, &LETTER_AT_96DPI).unwrap().page_count();
        prop_assert!(p_after >= p_before, "p_before={p_before} p_after={p_after}");
    }

    /// `dirty_rects` always has exactly one entry per page in Phase 4.1
    /// (full-content-area dirty hint; incremental narrowing comes later).
    #[test]
    fn dirty_rects_one_per_page(paragraphs in arb_paragraphs()) {
        let d = doc_from(&paragraphs);
        let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
        prop_assert_eq!(plan.dirty_rects.len(), plan.pages.len());
    }

    /// Every line's width fits inside the viewport's content width
    /// (modulo a 1-pixel rounding tolerance for the shaper). A line that
    /// overflows the content width is a layout bug — text would clip.
    #[test]
    fn line_widths_fit_content_width(paragraphs in arb_paragraphs()) {
        let d = doc_from(&paragraphs);
        let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
        let cw = LETTER_AT_96DPI.content_width();
        for page in &plan.pages {
            for bb in &page.blocks {
                for line in &bb.lines {
                    prop_assert!(
                        line.width_px <= cw + 1.0,
                        "line.width_px = {} > content_width + 1 = {}",
                        line.width_px,
                        cw + 1.0,
                    );
                }
            }
        }
    }

    /// Block kinds round-trip from doc-model to layout. For the simple
    /// strategy above (paragraphs only), every block in the plan must be
    /// `BlockKind::Paragraph`.
    #[test]
    fn paragraph_only_doc_yields_paragraph_kinds(paragraphs in arb_paragraphs()) {
        let d = doc_from(&paragraphs);
        let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
        for page in &plan.pages {
            for bb in &page.blocks {
                prop_assert_eq!(bb.kind, BlockKind::Paragraph);
            }
        }
    }
}
