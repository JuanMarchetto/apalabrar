//! Phase 5.1 RED — footnote bottom-of-page rendering + continuation.
//!
//! Locked semantics:
//! - `RenderPlan.footnote_refs` carries one entry per `\u{E001}`
//!   marker in body, in body (position-) order. `display_number` is
//!   1-indexed by that order (Word default).
//! - `Page.footnotes` carries `FootnoteBox`es pinned to the page's
//!   bottom shelf. Multiple footnotes on the same page stack
//!   top-down; a footnote that overflows produces an additional
//!   `FootnoteBox` on the next page with `is_continuation = true`.
//! - Footnote body uses 10pt font (smaller than 12pt body text);
//!   line height reflects this.

use apalabrar_doc_model::{Block, BlockKind, BlockTree, Doc, EditOp};
use apalabrar_layout::{LETTER_AT_96DPI, RenderPlan, layout};

fn paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph,
        text: text.into(),
    }
}

fn doc_with_one_footnote(body_text: &str, fn_text: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, body_text);
    d.apply_edit_op(EditOp::InsertFootnote {
        at: body_text.chars().count(),
        body: BlockTree {
            blocks: vec![paragraph(fn_text)],
        },
    })
    .unwrap();
    d
}

// ─────────────────────────────────────────────────────────────────
// RenderPlan default + empty cases
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_plan_default_has_empty_footnote_refs() {
    let plan = RenderPlan::default();
    assert!(plan.footnote_refs.is_empty());
}

#[test]
fn page_with_no_footnotes_has_empty_footnotes_vec() {
    let mut d = Doc::new();
    d.insert(0, "plain paragraph");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(plan.footnote_refs.is_empty());
    assert!(plan.pages[0].footnotes.is_empty());
}

// ─────────────────────────────────────────────────────────────────
// FootnoteRef — body-side anchor
// ─────────────────────────────────────────────────────────────────

#[test]
fn footnote_ref_emitted_for_each_marker() {
    let mut d = Doc::new();
    d.insert(0, "para one\npara two");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 3,
        body: BlockTree {
            blocks: vec![paragraph("a")],
        },
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 14, // inside para two (after the marker insert + \n shift)
        body: BlockTree {
            blocks: vec![paragraph("b")],
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.footnote_refs.len(), 2);
}

#[test]
fn footnote_ref_display_number_starts_at_one() {
    let d = doc_with_one_footnote("body", "fn1");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.footnote_refs.len(), 1);
    assert_eq!(plan.footnote_refs[0].display_number, 1);
}

#[test]
fn footnote_ref_display_numbers_are_consecutive_in_body_order() {
    let mut d = Doc::new();
    d.insert(0, "abc def ghi");
    // Insert at 3 first (body order: 1st), then at 0 (body order: 0th)
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 3,
        body: BlockTree {
            blocks: vec![paragraph("middle")],
        },
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: BlockTree {
            blocks: vec![paragraph("first")],
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let nums: Vec<usize> = plan
        .footnote_refs
        .iter()
        .map(|r| r.display_number)
        .collect();
    // Display numbers run 1, 2 in body-position order regardless of
    // insertion order.
    assert_eq!(nums, vec![1, 2]);
}

#[test]
fn footnote_ref_carries_page_index_zero_for_single_page_doc() {
    let d = doc_with_one_footnote("short", "fn");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.footnote_refs[0].page_index, 0);
}

#[test]
fn footnote_ref_carries_block_index_matching_marker_block() {
    // The marker is in the only paragraph (block 0) — a future test
    // covers refs across multiple blocks; this pins the simple case.
    let d = doc_with_one_footnote("text", "fn");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.footnote_refs[0].block_index, 0);
}

// ─────────────────────────────────────────────────────────────────
// FootnoteBox — bottom-of-page shelf
// ─────────────────────────────────────────────────────────────────

#[test]
fn page_with_one_footnote_emits_footnote_box() {
    let d = doc_with_one_footnote("body", "fn body");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.pages[0].footnotes.len(), 1);
    assert_eq!(plan.pages[0].footnotes[0].display_number, 1);
}

#[test]
fn footnote_box_origin_y_is_below_body_content() {
    let d = doc_with_one_footnote("body", "fn body");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let body_block = &plan.pages[0].blocks[0];
    let body_bottom = body_block.origin_y_px + body_block.height_px;
    let fn_box = &plan.pages[0].footnotes[0];
    assert!(
        fn_box.origin_y_px >= body_bottom,
        "footnote shelf must sit below body content: \
         body_bottom={body_bottom}, fn_origin_y={}",
        fn_box.origin_y_px,
    );
}

#[test]
fn footnote_box_uses_smaller_font_than_body() {
    // Footnote line height < body line height (10pt vs 12pt).
    let d = doc_with_one_footnote("body", "fn");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let body_line_h = plan.pages[0].blocks[0].lines[0].height_px;
    let fn_line_h = plan.pages[0].footnotes[0].lines[0].height_px;
    assert!(
        fn_line_h < body_line_h,
        "footnote line height ({fn_line_h}) must be smaller than body line height ({body_line_h})",
    );
}

#[test]
fn multiple_footnotes_on_same_page_stack_vertically() {
    let mut d = Doc::new();
    d.insert(0, "alpha beta");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 5,
        body: BlockTree {
            blocks: vec![paragraph("first footnote")],
        },
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 10,
        body: BlockTree {
            blocks: vec![paragraph("second footnote")],
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let fns = &plan.pages[0].footnotes;
    assert_eq!(fns.len(), 2);
    // Second footnote sits BELOW the first.
    assert!(
        fns[1].origin_y_px >= fns[0].origin_y_px + fns[0].height_px,
        "second footnote ({}) must stack below first ({}+{}={})",
        fns[1].origin_y_px,
        fns[0].origin_y_px,
        fns[0].height_px,
        fns[0].origin_y_px + fns[0].height_px,
    );
}

// ─────────────────────────────────────────────────────────────────
// Continuation across pages
// ─────────────────────────────────────────────────────────────────

#[test]
fn long_footnote_body_continues_to_next_page() {
    // A multi-paragraph footnote body that exceeds available shelf
    // space on one page must split: first segment on page 1, rest
    // on page 2 with `is_continuation = true`.
    let mut d = Doc::new();
    d.insert(0, "x"); // one tiny body block
    let many_paras: Vec<Block> = (0..200)
        .map(|i| {
            paragraph(&format!(
                "footnote line number {i} with enough text to wrap"
            ))
        })
        .collect();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 1,
        body: BlockTree { blocks: many_paras },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(
        plan.page_count() >= 2,
        "200-paragraph footnote should span ≥2 pages, got {}",
        plan.page_count(),
    );
    // At least one page after the first must have a footnote box.
    let any_continuation = plan.pages.iter().skip(1).any(|p| !p.footnotes.is_empty());
    assert!(
        any_continuation,
        "footnote spillover must produce footnote boxes on later pages",
    );
}

#[test]
fn footnote_continuation_carries_is_continuation_flag() {
    let mut d = Doc::new();
    d.insert(0, "x");
    let many_paras: Vec<Block> = (0..200)
        .map(|i| paragraph(&format!("line {i} with content for wrapping")))
        .collect();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 1,
        body: BlockTree { blocks: many_paras },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    // First-page footnote box: NOT a continuation.
    let first_box = plan.pages[0]
        .footnotes
        .first()
        .expect("page 1 has a footnote box");
    assert!(
        !first_box.is_continuation,
        "first segment must have is_continuation = false",
    );
    // Second-page footnote box: IS a continuation.
    let second_page_box = plan
        .pages
        .iter()
        .skip(1)
        .find_map(|p| p.footnotes.first())
        .expect("a later page has the continuation segment");
    assert!(
        second_page_box.is_continuation,
        "tail segment must have is_continuation = true",
    );
}

#[test]
fn each_page_footnotes_local_to_that_page_in_multi_page_doc() {
    // Sanity: in a tiny doc spanning multiple pages, a footnote whose
    // marker is on page N appears in page N's footnote shelf only —
    // not on every page.
    let mut d = Doc::new();
    // ~100 paragraphs to force pagination
    let body = (0..100)
        .map(|i| format!("paragraph number {i} with enough text"))
        .collect::<Vec<_>>()
        .join("\n");
    d.insert(0, &body);
    let body_len = d.text().chars().count();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: body_len, // marker at very end
        body: BlockTree {
            blocks: vec![paragraph("end-of-doc footnote")],
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(plan.page_count() >= 2);
    // The single FootnoteRef has page_index pointing to the last page.
    assert_eq!(plan.footnote_refs.len(), 1);
    let last_page_idx = plan.page_count() - 1;
    assert_eq!(plan.footnote_refs[0].page_index, last_page_idx);
    // Earlier pages have NO footnote boxes (the marker isn't on them).
    for page in &plan.pages[..last_page_idx] {
        assert!(
            page.footnotes.is_empty(),
            "page {} should have no footnotes (marker is on last page)",
            page.page_number,
        );
    }
    // Last page has the footnote box.
    assert!(!plan.pages[last_page_idx].footnotes.is_empty());
}
