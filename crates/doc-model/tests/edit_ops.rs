//! RED-phase tests for the EditOp dispatcher (Phase 2 prompt 2.2,
//! Phase A). The contract:
//!
//! - Variants 1-3 (InsertText / DeleteRange / FormatRange) run on
//!   the existing Loro text path. Happy paths + edge cases below.
//! - Variants 4-11 return `Err(Error::NotYetImplemented(name))`
//!   with the variant name in the payload — the bridge can branch
//!   on which ops are wired without exhaustively matching.
//! - Inverse property: `InsertText { at, text, marks: [] }` followed
//!   by `DeleteRange { from: at, to: at + chars(text) }` returns
//!   the doc to its prior text projection. Marked-text inverse is
//!   skipped because Loro's mark-removal isn't a standalone op in
//!   the blueprint yet.

use apalabrar_doc_model::{Block, BlockKind, BlockTree, Doc, EditOp, Error, Mark};
use proptest::prelude::*;

// ────────────────────────────────────────────────────────────────
// Variant 1 — InsertText
// ────────────────────────────────────────────────────────────────

#[test]
fn insert_text_happy_path_appends_text_to_empty_doc() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertText {
        at: 0,
        text: "hello".into(),
        marks: vec![],
    })
    .expect("insert succeeds");
    assert_eq!(d.text(), "hello");
}

#[test]
fn insert_text_at_middle_splices_into_existing() {
    let mut d = Doc::new();
    d.insert(0, "ad");
    d.apply_edit_op(EditOp::InsertText {
        at: 1,
        text: "bc".into(),
        marks: vec![],
    })
    .unwrap();
    assert_eq!(d.text(), "abcd");
}

#[test]
fn insert_text_with_marks_applies_them_to_inserted_span_only() {
    let mut d = Doc::new();
    d.insert(0, "abcde");
    d.apply_edit_op(EditOp::InsertText {
        at: 5,
        text: "FG".into(),
        marks: vec![Mark::Bold],
    })
    .unwrap();
    assert_eq!(d.text(), "abcdeFG");
    // The new span carries Bold.
    assert!(d.has_mark(5, Mark::Bold));
    assert!(d.has_mark(6, Mark::Bold));
    // The pre-existing span does NOT.
    assert!(!d.has_mark(0, Mark::Bold));
    assert!(!d.has_mark(4, Mark::Bold));
}

#[test]
fn insert_text_with_multiple_marks_applies_all_of_them() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertText {
        at: 0,
        text: "rich".into(),
        marks: vec![Mark::Bold, Mark::Italic],
    })
    .unwrap();
    assert!(d.has_mark(0, Mark::Bold));
    assert!(d.has_mark(0, Mark::Italic));
    assert!(d.has_mark(3, Mark::Bold));
    assert!(d.has_mark(3, Mark::Italic));
}

#[test]
fn insert_text_at_end_appends() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertText {
        at: 3,
        text: "de".into(),
        marks: vec![],
    })
    .unwrap();
    assert_eq!(d.text(), "abcde");
}

#[test]
fn insert_text_past_end_clips_to_end() {
    // Existing `Doc::insert` clips out-of-bounds indices defensively.
    // The dispatcher must inherit that contract — out-of-bounds at
    // value should not error, it should append at the actual length.
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertText {
        at: 999,
        text: "DE".into(),
        marks: vec![Mark::Italic],
    })
    .unwrap();
    assert_eq!(d.text(), "abcDE");
    // Marks land on the actual inserted span (3..5), not on the
    // garbage 999..1001 range.
    assert!(d.has_mark(3, Mark::Italic));
    assert!(d.has_mark(4, Mark::Italic));
    assert!(!d.has_mark(0, Mark::Italic));
}

#[test]
fn insert_text_with_empty_string_is_a_noop_even_with_marks() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertText {
        at: 1,
        text: "".into(),
        marks: vec![Mark::Bold],
    })
    .unwrap();
    assert_eq!(d.text(), "abc");
    // No span exists for the empty insert, so no codepoint should
    // carry the mark.
    assert!(!d.has_mark(0, Mark::Bold));
    assert!(!d.has_mark(1, Mark::Bold));
    assert!(!d.has_mark(2, Mark::Bold));
}

#[test]
fn insert_text_preserves_latam_codepoints() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertText {
        at: 0,
        text: "ñoño año mañana día sí".into(),
        marks: vec![Mark::Bold],
    })
    .unwrap();
    assert_eq!(d.text(), "ñoño año mañana día sí");
    // The ñ at codepoint 2 must carry Bold (no UTF-8 byte/codepoint
    // confusion at the dispatcher).
    assert!(d.has_mark(2, Mark::Bold));
}

// ────────────────────────────────────────────────────────────────
// Variant 2 — DeleteRange
// ────────────────────────────────────────────────────────────────

#[test]
fn delete_range_happy_path_removes_chars() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::DeleteRange { from: 5, to: 11 })
        .unwrap();
    assert_eq!(d.text(), "hello");
}

#[test]
fn delete_range_at_zero_removes_prefix() {
    let mut d = Doc::new();
    d.insert(0, "abcdef");
    d.apply_edit_op(EditOp::DeleteRange { from: 0, to: 3 })
        .unwrap();
    assert_eq!(d.text(), "def");
}

#[test]
fn delete_range_past_end_clips() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    // to=999 should clip to the actual length, removing only "bc".
    d.apply_edit_op(EditOp::DeleteRange { from: 1, to: 999 })
        .unwrap();
    assert_eq!(d.text(), "a");
}

#[test]
fn delete_range_from_greater_than_to_is_a_noop() {
    // The pre-existing `Doc::delete` clamps `end >= start`. The
    // dispatcher must inherit that contract — an inverted range
    // does NOT error and does NOT delete arbitrary text.
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::DeleteRange { from: 5, to: 1 })
        .unwrap();
    assert_eq!(d.text(), "abc");
}

#[test]
fn delete_range_empty_range_is_a_noop() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::DeleteRange { from: 1, to: 1 })
        .unwrap();
    assert_eq!(d.text(), "abc");
}

#[test]
fn delete_range_full_text_clears_doc() {
    let mut d = Doc::new();
    d.insert(0, "removeme");
    d.apply_edit_op(EditOp::DeleteRange { from: 0, to: 8 })
        .unwrap();
    assert_eq!(d.text(), "");
}

// ────────────────────────────────────────────────────────────────
// Variant 3 — FormatRange
// ────────────────────────────────────────────────────────────────

#[test]
fn format_range_happy_path_applies_mark() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::FormatRange {
        from: 0,
        to: 5,
        mark: Mark::Bold,
    })
    .unwrap();
    assert!(d.has_mark(0, Mark::Bold));
    assert!(d.has_mark(2, Mark::Bold));
    assert!(d.has_mark(4, Mark::Bold));
}

#[test]
fn format_range_partial_only_marks_inside_range() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::FormatRange {
        from: 1,
        to: 4,
        mark: Mark::Italic,
    })
    .unwrap();
    assert!(!d.has_mark(0, Mark::Italic));
    assert!(d.has_mark(1, Mark::Italic));
    assert!(d.has_mark(2, Mark::Italic));
    assert!(d.has_mark(3, Mark::Italic));
    assert!(!d.has_mark(4, Mark::Italic));
}

#[test]
fn format_range_empty_range_is_a_noop_with_no_mark_applied() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::FormatRange {
        from: 1,
        to: 1,
        mark: Mark::Bold,
    })
    .unwrap();
    assert!(!d.has_mark(0, Mark::Bold));
    assert!(!d.has_mark(1, Mark::Bold));
    assert!(!d.has_mark(2, Mark::Bold));
}

#[test]
fn format_range_past_end_clips() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::FormatRange {
        from: 0,
        to: 999,
        mark: Mark::Bold,
    })
    .unwrap();
    // Marks land on 0..3, the clipped range, not 0..999.
    assert!(d.has_mark(0, Mark::Bold));
    assert!(d.has_mark(2, Mark::Bold));
}

#[test]
fn format_range_compose_bold_and_italic() {
    let mut d = Doc::new();
    d.insert(0, "abcde");
    d.apply_edit_op(EditOp::FormatRange {
        from: 0,
        to: 5,
        mark: Mark::Bold,
    })
    .unwrap();
    d.apply_edit_op(EditOp::FormatRange {
        from: 1,
        to: 4,
        mark: Mark::Italic,
    })
    .unwrap();
    // Codepoints 1..4 carry both; codepoints 0 and 4 carry only Bold.
    assert!(d.has_mark(0, Mark::Bold) && !d.has_mark(0, Mark::Italic));
    assert!(d.has_mark(2, Mark::Bold) && d.has_mark(2, Mark::Italic));
    assert!(d.has_mark(4, Mark::Bold) && !d.has_mark(4, Mark::Italic));
}

// ────────────────────────────────────────────────────────────────
// Variants 4-11 — Phase B implements 4/5/6 (block model);
//                 7-11 still return NotYetImplemented.
// ────────────────────────────────────────────────────────────────

fn paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph,
        text: text.into(),
    }
}

fn heading(level: u8, text: &str) -> Block {
    Block {
        kind: BlockKind::Heading { level },
        text: text.into(),
    }
}

fn list_item(indent: u8, text: &str) -> Block {
    Block {
        kind: BlockKind::ListItem { indent },
        text: text.into(),
    }
}

// All EditOp variants are now implemented (Phases A-D ship complete);
// the prior `*_returns_not_yet_implemented` and `deferred_variants_*`
// pins are gone. The `Error::NotYetImplemented` variant remains in
// the public surface for forward-compat (any future EditOp variant
// added to the enum has a place to land while its handler is being
// written), but no current variant produces it.

// ────────────────────────────────────────────────────────────────
// Property: InsertText then its inverse DeleteRange returns to prior
// text projection. Generated parameters are clipped at apply time so
// the property holds even for at > prior_len.
// ────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_insert_then_delete_inverse_restores_text(
        initial in "[a-zA-Z0-9 ]{0,40}",
        at in 0usize..50,
        inserted in "[a-zA-Z0-9ñáéíóúüç ]{0,20}",
    ) {
        let mut d = Doc::new();
        d.insert(0, &initial);
        let prior = d.text();
        let prior_len = prior.chars().count();

        let actual_at = at.min(prior_len);
        let chars_inserted = inserted.chars().count();

        d.apply_edit_op(EditOp::InsertText {
            at,
            text: inserted,
            marks: vec![],
        }).unwrap();

        d.apply_edit_op(EditOp::DeleteRange {
            from: actual_at,
            to: actual_at + chars_inserted,
        }).unwrap();

        prop_assert_eq!(d.text(), prior);
    }

    /// Empty-text insert is a no-op; the doc text is unchanged
    /// regardless of `at` and the marks list.
    #[test]
    fn prop_empty_text_insert_is_noop(
        initial in "[a-zA-Z ]{0,30}",
        at in 0usize..50,
    ) {
        let mut d = Doc::new();
        d.insert(0, &initial);
        let prior = d.text();
        d.apply_edit_op(EditOp::InsertText {
            at,
            text: "".into(),
            marks: vec![Mark::Bold, Mark::Italic],
        }).unwrap();
        prop_assert_eq!(d.text(), prior);
    }

    // Note: `prop_deferred_variants_preserve_text` retired in Phase D
    // — every EditOp variant now has a real handler.
}

// ────────────────────────────────────────────────────────────────
// Phase B — block accessors (block_count, block)
// ────────────────────────────────────────────────────────────────

#[test]
fn block_count_empty_doc_is_one() {
    let d = Doc::new();
    // The implicit empty paragraph counts as one block.
    assert_eq!(d.block_count(), 1);
}

#[test]
fn block_count_single_text_no_newlines_is_one() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    assert_eq!(d.block_count(), 1);
}

#[test]
fn block_count_phase_a_insert_with_newlines() {
    // Phase A's `insert` is the raw path. block_count() must reflect
    // the body's `\n` count regardless of how the chars got there.
    let mut d = Doc::new();
    d.insert(0, "alpha\nbeta\ngamma");
    assert_eq!(d.block_count(), 3);
}

#[test]
fn block_at_zero_returns_first_segment() {
    let mut d = Doc::new();
    d.insert(0, "hello\nworld");
    let b = d.block(0).expect("block 0 exists");
    assert_eq!(b.text, "hello");
    assert_eq!(b.kind, BlockKind::Paragraph);
}

#[test]
fn block_at_last_index_returns_trailing_segment() {
    let mut d = Doc::new();
    d.insert(0, "hello\nworld");
    let b = d.block(1).expect("block 1 exists");
    assert_eq!(b.text, "world");
    assert_eq!(b.kind, BlockKind::Paragraph);
}

#[test]
fn block_out_of_range_returns_none() {
    let d = Doc::new();
    assert!(d.block(1).is_none());
    assert!(d.block(999).is_none());
}

#[test]
fn block_in_empty_doc_returns_empty_paragraph() {
    let d = Doc::new();
    let b = d.block(0).expect("block 0 of empty doc exists");
    assert_eq!(b.text, "");
    assert_eq!(b.kind, BlockKind::Paragraph);
}

// ────────────────────────────────────────────────────────────────
// Phase B — InsertBlock variant
// ────────────────────────────────────────────────────────────────

#[test]
fn insert_block_into_empty_doc_creates_two_blocks() {
    // Empty doc has 1 implicit empty paragraph. Inserting a heading
    // at pos 0 should yield 2 blocks: [Heading "intro", Paragraph ""]
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(1, "intro"),
    })
    .unwrap();
    assert_eq!(d.block_count(), 2);
    let b0 = d.block(0).unwrap();
    assert_eq!(b0.text, "intro");
    assert_eq!(b0.kind, BlockKind::Heading { level: 1 });
    let b1 = d.block(1).unwrap();
    assert_eq!(b1.text, "");
    assert_eq!(b1.kind, BlockKind::Paragraph);
}

#[test]
fn insert_block_at_start_of_existing_block_prepends() {
    // Doc has "hello" (1 paragraph). Insert heading "title" at pos 0.
    // Expect: [Heading "title", Paragraph "hello"].
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(2, "title"),
    })
    .unwrap();
    assert_eq!(d.text(), "title\nhello");
    assert_eq!(d.block_count(), 2);
    let b0 = d.block(0).unwrap();
    assert_eq!(b0.text, "title");
    assert_eq!(b0.kind, BlockKind::Heading { level: 2 });
    let b1 = d.block(1).unwrap();
    assert_eq!(b1.text, "hello");
    assert_eq!(b1.kind, BlockKind::Paragraph);
}

#[test]
fn insert_block_at_end_of_existing_block_appends() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 5,
        block: heading(3, "footer"),
    })
    .unwrap();
    assert_eq!(d.text(), "hello\nfooter");
    assert_eq!(d.block_count(), 2);
    let b1 = d.block(1).unwrap();
    assert_eq!(b1.text, "footer");
    assert_eq!(b1.kind, BlockKind::Heading { level: 3 });
    // Original block 0 keeps its kind.
    let b0 = d.block(0).unwrap();
    assert_eq!(b0.kind, BlockKind::Paragraph);
}

#[test]
fn insert_block_mid_existing_block_splits_into_three() {
    // Doc has "abcdef" (1 block). Insert heading "X" at pos 3.
    // Expect: [Paragraph "abc", Heading "X", Paragraph "def"]
    let mut d = Doc::new();
    d.insert(0, "abcdef");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 3,
        block: heading(1, "X"),
    })
    .unwrap();
    assert_eq!(d.text(), "abc\nX\ndef");
    assert_eq!(d.block_count(), 3);
    assert_eq!(d.block(0).unwrap().text, "abc");
    assert_eq!(d.block(0).unwrap().kind, BlockKind::Paragraph);
    assert_eq!(d.block(1).unwrap().text, "X");
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Heading { level: 1 });
    assert_eq!(d.block(2).unwrap().text, "def");
    assert_eq!(d.block(2).unwrap().kind, BlockKind::Paragraph);
}

#[test]
fn insert_block_mid_block_preserves_split_kind_on_after_half() {
    // Doc has heading "aaabbb" (1 heading). Insert paragraph "X" at pos 3.
    // Expect: [Heading "aaa", Paragraph "X", Heading "bbb"]
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(2, "aaabbb"),
    })
    .unwrap();
    // Now blocks are [Heading "aaabbb", Paragraph ""].
    // Insert paragraph "X" at pos 3 (mid-heading).
    d.apply_edit_op(EditOp::InsertBlock {
        at: 3,
        block: paragraph("X"),
    })
    .unwrap();
    // Expect: [Heading "aaa", Paragraph "X", Heading "bbb", Paragraph ""].
    assert_eq!(d.block_count(), 4);
    assert_eq!(d.block(0).unwrap().kind, BlockKind::Heading { level: 2 });
    assert_eq!(d.block(0).unwrap().text, "aaa");
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Paragraph);
    assert_eq!(d.block(1).unwrap().text, "X");
    assert_eq!(d.block(2).unwrap().kind, BlockKind::Heading { level: 2 });
    assert_eq!(d.block(2).unwrap().text, "bbb");
    assert_eq!(d.block(3).unwrap().kind, BlockKind::Paragraph);
}

#[test]
fn insert_block_with_listitem_preserves_indent() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: list_item(2, "item"),
    })
    .unwrap();
    let b = d.block(0).unwrap();
    assert_eq!(b.kind, BlockKind::ListItem { indent: 2 });
    assert_eq!(b.text, "item");
}

#[test]
fn insert_block_clamps_heading_level_to_six() {
    // Headings only support 1..=6. A level=99 should clamp to 6.
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(99, "title"),
    })
    .unwrap();
    let b = d.block(0).unwrap();
    assert_eq!(b.kind, BlockKind::Heading { level: 6 });
}

#[test]
fn insert_block_clamps_heading_level_zero_to_one() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(0, "title"),
    })
    .unwrap();
    let b = d.block(0).unwrap();
    assert_eq!(b.kind, BlockKind::Heading { level: 1 });
}

#[test]
fn insert_block_with_empty_text_creates_empty_block() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: paragraph(""),
    })
    .unwrap();
    assert_eq!(d.text(), "\nhello");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(0).unwrap().text, "");
}

#[test]
fn insert_block_past_end_clips_to_end() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 999,
        block: heading(1, "X"),
    })
    .unwrap();
    // Out-of-bounds clips to end-of-doc, appending the new block.
    assert_eq!(d.text(), "abc\nX");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Heading { level: 1 });
}

#[test]
fn insert_block_at_end_of_last_block_in_multi_block_doc() {
    // Tests the block_range / offset_in_block / block_len math when
    // block_idx > 0. With block_idx=2 in "ab\ncd\nef", block_start=6
    // and block_end=8 — both must compute correctly for the END
    // branch to fire (offset_in_block == block_len).
    let mut d = Doc::new();
    d.insert(0, "ab\ncd\nef");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 8,
        block: heading(4, "X"),
    })
    .unwrap();
    assert_eq!(d.text(), "ab\ncd\nef\nX");
    assert_eq!(d.block_count(), 4);
    assert_eq!(d.block(3).unwrap().kind, BlockKind::Heading { level: 4 });
    assert_eq!(d.block(0).unwrap().text, "ab");
    assert_eq!(d.block(1).unwrap().text, "cd");
    assert_eq!(d.block(2).unwrap().text, "ef");
}

#[test]
fn insert_block_mid_of_non_first_block_preserves_neighbors() {
    // Tests block_range / offset_in_block / block_len when MID
    // branch fires for block_idx > 0. Splits a non-first heading
    // into two halves with a paragraph between.
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: paragraph("ab"),
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 3,
        block: heading(2, "cdef"),
    })
    .unwrap();
    // Body now: "ab\ncdef\n", blocks: [Para "ab", Heading "cdef", Para ""].
    // Split the heading mid-text: pos 5 is between 'd' and 'e' inside block 1.
    d.apply_edit_op(EditOp::InsertBlock {
        at: 5,
        block: paragraph("X"),
    })
    .unwrap();
    // Expected: [Para "ab", Heading "cd", Para "X", Heading "ef", Para ""]
    assert_eq!(d.block_count(), 5);
    assert_eq!(d.block(0).unwrap().text, "ab");
    assert_eq!(d.block(0).unwrap().kind, BlockKind::Paragraph);
    assert_eq!(d.block(1).unwrap().text, "cd");
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Heading { level: 2 });
    assert_eq!(d.block(2).unwrap().text, "X");
    assert_eq!(d.block(2).unwrap().kind, BlockKind::Paragraph);
    assert_eq!(d.block(3).unwrap().text, "ef");
    assert_eq!(d.block(3).unwrap().kind, BlockKind::Heading { level: 2 });
    assert_eq!(d.block(4).unwrap().text, "");
    assert_eq!(d.block(4).unwrap().kind, BlockKind::Paragraph);
}

#[test]
fn insert_block_preserves_latam_codepoints() {
    let mut d = Doc::new();
    d.insert(0, "año");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 1,
        block: heading(1, "ñoño"),
    })
    .unwrap();
    // Original "año" splits into "a" and "ño" around the heading.
    assert_eq!(d.text(), "a\nñoño\nño");
    assert_eq!(d.block_count(), 3);
    assert_eq!(d.block(0).unwrap().text, "a");
    assert_eq!(d.block(1).unwrap().text, "ñoño");
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Heading { level: 1 });
    assert_eq!(d.block(2).unwrap().text, "ño");
}

// ────────────────────────────────────────────────────────────────
// Phase B — SplitBlock variant
// ────────────────────────────────────────────────────────────────

#[test]
fn split_block_at_middle_creates_two_blocks() {
    let mut d = Doc::new();
    d.insert(0, "abcdef");
    d.apply_edit_op(EditOp::SplitBlock { at: 3 }).unwrap();
    assert_eq!(d.text(), "abc\ndef");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(0).unwrap().text, "abc");
    assert_eq!(d.block(1).unwrap().text, "def");
    // Both halves keep the original Paragraph kind.
    assert_eq!(d.block(0).unwrap().kind, BlockKind::Paragraph);
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Paragraph);
}

#[test]
fn split_block_at_start_creates_empty_first_block() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::SplitBlock { at: 0 }).unwrap();
    assert_eq!(d.text(), "\nabc");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(0).unwrap().text, "");
    assert_eq!(d.block(1).unwrap().text, "abc");
}

#[test]
fn split_block_at_end_creates_empty_last_block() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::SplitBlock { at: 3 }).unwrap();
    assert_eq!(d.text(), "abc\n");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(0).unwrap().text, "abc");
    assert_eq!(d.block(1).unwrap().text, "");
}

#[test]
fn split_block_inherits_kind_for_both_halves() {
    // Insert a Heading, then split it. Both halves should be Heading.
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(2, "abcdef"),
    })
    .unwrap();
    d.apply_edit_op(EditOp::SplitBlock { at: 3 }).unwrap();
    // Now blocks: [Heading "abc", Heading "def", Paragraph ""].
    assert_eq!(d.block_count(), 3);
    assert_eq!(d.block(0).unwrap().kind, BlockKind::Heading { level: 2 });
    assert_eq!(d.block(0).unwrap().text, "abc");
    assert_eq!(d.block(1).unwrap().kind, BlockKind::Heading { level: 2 });
    assert_eq!(d.block(1).unwrap().text, "def");
}

#[test]
fn split_block_past_end_clips() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::SplitBlock { at: 999 }).unwrap();
    // Out-of-bounds clips to end → split at position 3.
    assert_eq!(d.text(), "abc\n");
    assert_eq!(d.block_count(), 2);
}

#[test]
fn split_block_at_non_first_block_position() {
    // Tests SplitBlock when block_idx > 0 (exercises non-zero
    // block_idx codepath in the body insert and blocks list insert).
    let mut d = Doc::new();
    d.insert(0, "a\nbcd\ne");
    // body: "a\nbcd\ne", positions: a=0, \n=1, b=2, c=3, d=4, \n=5, e=6.
    // Split inside block 1 ("bcd") at pos 3 ('c').
    d.apply_edit_op(EditOp::SplitBlock { at: 3 }).unwrap();
    assert_eq!(d.text(), "a\nb\ncd\ne");
    assert_eq!(d.block_count(), 4);
    assert_eq!(d.block(0).unwrap().text, "a");
    assert_eq!(d.block(1).unwrap().text, "b");
    assert_eq!(d.block(2).unwrap().text, "cd");
    assert_eq!(d.block(3).unwrap().text, "e");
}

#[test]
fn split_block_in_empty_doc_creates_two_empty_blocks() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::SplitBlock { at: 0 }).unwrap();
    assert_eq!(d.text(), "\n");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(0).unwrap().text, "");
    assert_eq!(d.block(1).unwrap().text, "");
}

// ────────────────────────────────────────────────────────────────
// Phase B — MergeBlocks variant
// ────────────────────────────────────────────────────────────────

#[test]
fn merge_blocks_adjacent_combines_text() {
    let mut d = Doc::new();
    d.insert(0, "hello\nworld");
    // first=0 (in block 0), second=6 (in block 1) → adjacent.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 6,
    })
    .unwrap();
    assert_eq!(d.text(), "helloworld");
    assert_eq!(d.block_count(), 1);
}

#[test]
fn merge_blocks_first_kind_wins() {
    // [Heading "ab", Paragraph "cd"] → merge → Heading "abcd"
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(1, "ab"),
    })
    .unwrap();
    // Doc is now [Heading "ab", Paragraph ""]. Append a paragraph.
    d.insert(3, "cd"); // body becomes "ab\ncd"
    // Doc is now [Heading "ab", Paragraph "cd"].
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 4,
    })
    .unwrap();
    assert_eq!(d.text(), "abcd");
    assert_eq!(d.block_count(), 1);
    let b = d.block(0).unwrap();
    assert_eq!(b.kind, BlockKind::Heading { level: 1 });
    assert_eq!(b.text, "abcd");
}

#[test]
fn merge_blocks_non_adjacent_is_noop() {
    let mut d = Doc::new();
    d.insert(0, "a\nb\nc");
    let prior_text = d.text();
    let prior_count = d.block_count();
    // first=0 (block 0), second=4 (block 2) → not adjacent.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 4,
    })
    .unwrap();
    assert_eq!(d.text(), prior_text);
    assert_eq!(d.block_count(), prior_count);
}

#[test]
fn merge_blocks_same_block_is_noop() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    // both positions in block 0.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 2,
    })
    .unwrap();
    assert_eq!(d.text(), "abc");
    assert_eq!(d.block_count(), 1);
}

#[test]
fn merge_blocks_in_empty_doc_is_noop() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 1,
    })
    .unwrap();
    assert_eq!(d.text(), "");
    assert_eq!(d.block_count(), 1);
}

#[test]
fn merge_blocks_at_non_first_indices() {
    // Tests the merge loop counter on a non-first \n: i_first=1
    // forces the loop to advance the block counter past the first
    // \n before finding the boundary to delete. Catches mutations
    // that freeze or reverse the counter increment.
    let mut d = Doc::new();
    d.insert(0, "a\nb\nc");
    // first=2 (block 1 "b"), second=4 (block 2 "c"). i_first=1.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 2,
        second: 4,
    })
    .unwrap();
    assert_eq!(d.text(), "a\nbc");
    assert_eq!(d.block_count(), 2);
    assert_eq!(d.block(0).unwrap().text, "a");
    assert_eq!(d.block(1).unwrap().text, "bc");
}

#[test]
fn merge_blocks_at_indices_two_and_three() {
    // Even higher i_first to force at least two loop increments.
    let mut d = Doc::new();
    d.insert(0, "a\nb\nc\nd");
    // first=4 (block 2 "c"), second=6 (block 3 "d"). i_first=2.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 4,
        second: 6,
    })
    .unwrap();
    assert_eq!(d.text(), "a\nb\ncd");
    assert_eq!(d.block_count(), 3);
    assert_eq!(d.block(0).unwrap().text, "a");
    assert_eq!(d.block(1).unwrap().text, "b");
    assert_eq!(d.block(2).unwrap().text, "cd");
}

#[test]
fn merge_blocks_clips_out_of_bounds_positions() {
    let mut d = Doc::new();
    d.insert(0, "ab\ncd");
    // second=999 clips to len=5, which is in block 1. first=0 in block 0. Adjacent → merge.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 999,
    })
    .unwrap();
    assert_eq!(d.text(), "abcd");
    assert_eq!(d.block_count(), 1);
}

// ────────────────────────────────────────────────────────────────
// Phase B — Snapshot round-trip preserves block kinds
// ────────────────────────────────────────────────────────────────

#[test]
fn snapshot_preserves_block_kinds() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: heading(1, "title"),
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 5,
        block: list_item(2, "item-A"),
    })
    .unwrap();
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    assert_eq!(restored.block_count(), d.block_count());
    for i in 0..d.block_count() {
        let original = d.block(i).expect("block exists");
        let copied = restored.block(i).expect("block exists in restored");
        assert_eq!(original.text, copied.text);
        assert_eq!(original.kind, copied.kind);
    }
}

// ────────────────────────────────────────────────────────────────
// Phase B properties
// ────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// `block_count()` always equals the number of `\n` codepoints in
    /// the body plus one. Holds across arbitrary mixed Phase A and
    /// Phase B operations.
    #[test]
    fn prop_block_count_matches_newlines_plus_one(
        text in "[a-z\n]{0,40}",
    ) {
        let mut d = Doc::new();
        d.insert(0, &text);
        let expected = text.chars().filter(|c| *c == '\n').count() + 1;
        prop_assert_eq!(d.block_count(), expected);
    }

    /// SplitBlock followed by MergeBlocks at the same boundary
    /// returns to the prior text. Block count and text both restore.
    #[test]
    fn prop_split_then_merge_restores_text(
        prefix in "[a-z]{1,10}",
        suffix in "[a-z]{1,10}",
    ) {
        let mut d = Doc::new();
        let combined = format!("{prefix}{suffix}");
        d.insert(0, &combined);
        let prior_text = d.text();
        let prior_count = d.block_count();
        let split_at = prefix.chars().count();
        d.apply_edit_op(EditOp::SplitBlock { at: split_at }).unwrap();
        prop_assert_eq!(d.block_count(), prior_count + 1);
        // After split, `split_at` is at the end of block 0; the \n
        // is at position `split_at`. Position split_at+1 is start of
        // block 1. Merge them.
        d.apply_edit_op(EditOp::MergeBlocks {
            first: 0,
            second: split_at + 1,
        }).unwrap();
        prop_assert_eq!(d.text(), prior_text);
        prop_assert_eq!(d.block_count(), prior_count);
    }

    /// Each InsertBlock increments block_count by exactly one.
    #[test]
    fn prop_insert_block_increments_count(
        initial in "[a-z]{0,20}",
        n_inserts in 1usize..6,
        kind_marker in 0u8..3,
    ) {
        let mut d = Doc::new();
        d.insert(0, &initial);
        let prior_count = d.block_count();
        let block = match kind_marker {
            0 => paragraph("X"),
            1 => heading(2, "X"),
            _ => list_item(0, "X"),
        };
        for _ in 0..n_inserts {
            d.apply_edit_op(EditOp::InsertBlock { at: 0, block: block.clone() }).unwrap();
        }
        prop_assert_eq!(d.block_count(), prior_count + n_inserts);
    }
}

// ────────────────────────────────────────────────────────────────
// Phase C — InsertComment variant
// ────────────────────────────────────────────────────────────────

use apalabrar_doc_model::SuggestionState;

#[test]
fn insert_comment_with_explicit_thread_id_stores_record() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 5,
        body: "fix this".into(),
        thread_id: Some("t-explicit".into()),
    })
    .unwrap();
    let c = d.comment("t-explicit").expect("comment present");
    assert_eq!(c.thread_id, "t-explicit");
    assert_eq!(c.from, 0);
    assert_eq!(c.to, 5);
    assert_eq!(c.body, "fix this");
}

#[test]
fn insert_comment_without_thread_id_generates_one() {
    let mut d = Doc::new();
    d.insert(0, "hi");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 2,
        body: "review".into(),
        thread_id: None,
    })
    .unwrap();
    let assigned = d
        .last_comment_thread_id()
        .expect("last_comment_thread_id should be set");
    assert!(!assigned.is_empty());
    let c = d.comment(&assigned).expect("comment present");
    assert_eq!(c.body, "review");
}

#[test]
fn insert_comment_assigns_strictly_increasing_counter_suffixes() {
    // Catches a mutation in `next_id` that would flip the counter
    // direction (n - 1 instead of n + 1). Both directions produce
    // distinct ids, so a unique-id check isn't enough — we need to
    // assert the suffix integer ORDERING.
    let mut d = Doc::new();
    let mut suffixes: Vec<i64> = Vec::new();
    for _ in 0..3 {
        d.apply_edit_op(EditOp::InsertComment {
            from: 0,
            to: 0,
            body: "x".into(),
            thread_id: None,
        })
        .unwrap();
        let id = d.last_comment_thread_id().unwrap();
        // Format: "c-{peer_hex}-{counter}". `splitn(3, '-')` picks up
        // the counter as a single segment even if it has a leading
        // '-' (negative). A naive `rsplit('-')` would treat the
        // sign as a separator and lose it, masking the mutation.
        let parts: Vec<&str> = id.splitn(3, '-').collect();
        assert_eq!(parts.len(), 3, "id format must be prefix-peer-counter");
        let suffix: i64 = parts[2]
            .parse()
            .expect("counter segment must be a parseable i64");
        suffixes.push(suffix);
    }
    // Strictly increasing — catches the n+1 → n-1 mutation.
    assert!(
        suffixes[0] < suffixes[1] && suffixes[1] < suffixes[2],
        "expected strictly increasing counter suffixes, got {suffixes:?}"
    );
}

#[test]
fn insert_comment_generates_unique_ids_across_calls() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    let mut ids = Vec::new();
    for _ in 0..5 {
        d.apply_edit_op(EditOp::InsertComment {
            from: 0,
            to: 1,
            body: "body".into(),
            thread_id: None,
        })
        .unwrap();
        ids.push(d.last_comment_thread_id().unwrap());
    }
    // All five ids must be distinct.
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), 5);
}

#[test]
fn insert_comment_explicit_id_overrides_generated() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 1,
        body: "body".into(),
        thread_id: Some("user-supplied".into()),
    })
    .unwrap();
    assert_eq!(d.last_comment_thread_id().as_deref(), Some("user-supplied"));
}

#[test]
fn insert_comment_lists_thread_id() {
    let mut d = Doc::new();
    d.insert(0, "a");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 1,
        body: "body".into(),
        thread_id: Some("thread-1".into()),
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 1,
        body: "second".into(),
        thread_id: Some("thread-2".into()),
    })
    .unwrap();
    let mut ids = d.comment_thread_ids();
    ids.sort();
    assert_eq!(ids, vec!["thread-1".to_string(), "thread-2".to_string()]);
}

#[test]
fn insert_comment_empty_doc_lists_no_threads() {
    let d = Doc::new();
    assert!(d.comment_thread_ids().is_empty());
    assert!(d.comment("anything").is_none());
}

#[test]
fn insert_comment_preserves_latam_codepoints_in_body() {
    let mut d = Doc::new();
    d.insert(0, "año");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 3,
        body: "ñoño año mañana".into(),
        thread_id: Some("ñ".into()),
    })
    .unwrap();
    let c = d.comment("ñ").unwrap();
    assert_eq!(c.body, "ñoño año mañana");
}

// ────────────────────────────────────────────────────────────────
// Phase C — Suggest variant
// ────────────────────────────────────────────────────────────────

#[test]
fn suggest_creates_pending_record() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 5,
        replacement: "HOLA".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().expect("id assigned");
    let s = d.suggestion(&id).expect("suggestion present");
    assert_eq!(s.id, id);
    assert_eq!(s.from, 0);
    assert_eq!(s.to, 5);
    assert_eq!(s.replacement, "HOLA");
    assert_eq!(s.state, SuggestionState::Pending);
    // Doc text is NOT mutated yet — Suggest is a proposal, not an apply.
    assert_eq!(d.text(), "hello world");
}

#[test]
fn suggest_generates_unique_ids_across_calls() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    let mut ids = Vec::new();
    for _ in 0..5 {
        d.apply_edit_op(EditOp::Suggest {
            from: 0,
            to: 1,
            replacement: "X".into(),
        })
        .unwrap();
        ids.push(d.last_suggestion_id().unwrap());
    }
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), 5);
}

#[test]
fn suggest_lists_in_pending_ids() {
    let mut d = Doc::new();
    d.insert(0, "abcdef");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 1,
        replacement: "X".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    assert_eq!(d.pending_suggestion_ids(), vec![id.clone()]);
    assert_eq!(d.suggestion_ids(), vec![id]);
}

#[test]
fn suggest_with_empty_replacement_is_a_proposed_deletion() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 5,
        to: 11,
        replacement: "".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    let s = d.suggestion(&id).unwrap();
    assert_eq!(s.from, 5);
    assert_eq!(s.to, 11);
    assert_eq!(s.replacement, "");
    assert_eq!(s.state, SuggestionState::Pending);
}

// ────────────────────────────────────────────────────────────────
// Phase C — AcceptSuggestion variant
// ────────────────────────────────────────────────────────────────

#[test]
fn accept_suggestion_applies_replacement_and_marks_accepted() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 5,
        replacement: "HOLA".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.text(), "HOLA world");
    let s = d.suggestion(&id).unwrap();
    assert_eq!(s.state, SuggestionState::Accepted);
    // The accepted suggestion is no longer pending.
    assert!(!d.pending_suggestion_ids().contains(&id));
}

#[test]
fn accept_suggestion_with_empty_replacement_deletes_range() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 5,
        to: 11,
        replacement: "".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.text(), "hello");
    assert_eq!(d.suggestion(&id).unwrap().state, SuggestionState::Accepted);
}

#[test]
fn accept_suggestion_unknown_id_returns_error() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    let result = d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: "nope".into(),
    });
    assert_eq!(result, Err(Error::SuggestionNotFound("nope".into())));
    // Doc text untouched.
    assert_eq!(d.text(), "abc");
}

#[test]
fn accept_suggestion_already_accepted_is_idempotent() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 1,
        replacement: "X".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    let text_after_first = d.text();
    // Second accept must be a no-op (returns Ok, doesn't re-apply).
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.text(), text_after_first);
    assert_eq!(d.suggestion(&id).unwrap().state, SuggestionState::Accepted);
}

#[test]
fn accept_suggestion_preserves_other_pending() {
    // Two pending suggestions; accept one. The other stays pending.
    let mut d = Doc::new();
    d.insert(0, "AAAA BBBB");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 4,
        replacement: "XX".into(),
    })
    .unwrap();
    let id1 = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::Suggest {
        from: 5,
        to: 9,
        replacement: "YY".into(),
    })
    .unwrap();
    let id2 = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id1.clone(),
    })
    .unwrap();
    // id1 is accepted; id2 is still pending.
    assert_eq!(d.suggestion(&id1).unwrap().state, SuggestionState::Accepted);
    assert_eq!(d.suggestion(&id2).unwrap().state, SuggestionState::Pending);
    assert_eq!(d.pending_suggestion_ids(), vec![id2]);
}

// ────────────────────────────────────────────────────────────────
// Phase C — Snapshot durability
// ────────────────────────────────────────────────────────────────

#[test]
fn snapshot_round_trip_preserves_comments() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 5,
        body: "review".into(),
        thread_id: Some("t1".into()),
    })
    .unwrap();
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    let c = restored.comment("t1").expect("comment survives snapshot");
    assert_eq!(c.from, 0);
    assert_eq!(c.to, 5);
    assert_eq!(c.body, "review");
}

#[test]
fn snapshot_round_trip_preserves_suggestion_state() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 5,
        replacement: "HOLA".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    let s = restored.suggestion(&id).expect("suggestion survives");
    assert_eq!(s.state, SuggestionState::Pending);
    assert_eq!(s.replacement, "HOLA");
    // Pending list intact post-snapshot.
    assert_eq!(restored.pending_suggestion_ids(), vec![id]);
}

#[test]
fn snapshot_round_trip_preserves_accepted_suggestion() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 5,
        replacement: "HOLA".into(),
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    assert_eq!(restored.text(), "HOLA world");
    assert_eq!(
        restored.suggestion(&id).unwrap().state,
        SuggestionState::Accepted
    );
}

// ────────────────────────────────────────────────────────────────
// Phase C properties
// ────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Each `Suggest` followed by `AcceptSuggestion(its id)` must
    /// transform the doc text identically to a manual delete+insert
    /// of `[from, to)` → `replacement`. The cleaner property is text-
    /// equivalence to the manual splice.
    #[test]
    fn prop_accept_equals_manual_splice(
        prefix in "[a-z]{0,8}",
        target in "[a-z]{1,8}",
        suffix in "[a-z]{0,8}",
        replacement in "[A-Z]{0,8}",
    ) {
        let mut suggested = Doc::new();
        suggested.insert(0, &format!("{prefix}{target}{suffix}"));
        let from = prefix.chars().count();
        let to = from + target.chars().count();
        suggested.apply_edit_op(EditOp::Suggest {
            from, to, replacement: replacement.clone(),
        }).unwrap();
        let id = suggested.last_suggestion_id().unwrap();
        suggested.apply_edit_op(EditOp::AcceptSuggestion {
            suggestion_id: id,
        }).unwrap();

        let mut manual = Doc::new();
        manual.insert(0, &format!("{prefix}{target}{suffix}"));
        manual.delete(from..to);
        manual.insert(from, &replacement);

        prop_assert_eq!(suggested.text(), manual.text());
    }

    /// `comment_thread_ids().len()` equals the number of distinct
    /// thread_ids inserted (no duplicates).
    #[test]
    fn prop_comment_thread_count_matches_inserts(
        n in 1usize..6,
    ) {
        let mut d = Doc::new();
        d.insert(0, "abc");
        for i in 0..n {
            d.apply_edit_op(EditOp::InsertComment {
                from: 0, to: 1, body: "b".into(),
                thread_id: Some(format!("thread-{i}")),
            }).unwrap();
        }
        prop_assert_eq!(d.comment_thread_ids().len(), n);
    }

    /// `pending_suggestion_ids().len()` decreases by exactly one
    /// after each `AcceptSuggestion`.
    #[test]
    fn prop_accept_decrements_pending_count(
        n_pending in 2usize..6,
    ) {
        let mut d = Doc::new();
        d.insert(0, &"a".repeat(n_pending * 2));
        let mut ids = Vec::new();
        for i in 0..n_pending {
            d.apply_edit_op(EditOp::Suggest {
                from: i * 2, to: i * 2 + 1, replacement: "X".into(),
            }).unwrap();
            ids.push(d.last_suggestion_id().unwrap());
        }
        let mut expected = n_pending;
        prop_assert_eq!(d.pending_suggestion_ids().len(), expected);
        for id in &ids {
            d.apply_edit_op(EditOp::AcceptSuggestion {
                suggestion_id: id.clone(),
            }).unwrap();
            expected -= 1;
            prop_assert_eq!(d.pending_suggestion_ids().len(), expected);
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Phase D — InsertCitation variant
// ────────────────────────────────────────────────────────────────

const CITATION_MARKER: char = '\u{E000}';
const FOOTNOTE_MARKER: char = '\u{E001}';

#[test]
fn insert_citation_inserts_marker_codepoint_at_position() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 5,
        key: "Smith2020".into(),
    })
    .unwrap();
    // Body now ends with the marker codepoint.
    let body = d.text();
    let chars: Vec<char> = body.chars().collect();
    assert_eq!(chars.len(), 6);
    assert_eq!(chars[5], CITATION_MARKER);
    assert_eq!(&body[..5], "hello");
}

#[test]
fn insert_citation_records_key_and_position() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 5,
        key: "Smith2020".into(),
    })
    .unwrap();
    let id = d.last_citation_id().expect("id assigned");
    let c = d.citation(&id).expect("citation present");
    assert_eq!(c.id, id);
    assert_eq!(c.at, 5);
    assert_eq!(c.key, "Smith2020");
}

#[test]
fn insert_citation_at_zero_prepends_marker() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 0,
        key: "K".into(),
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    assert_eq!(chars[0], CITATION_MARKER);
    assert_eq!(chars[1..].iter().collect::<String>(), "hello");
}

#[test]
fn insert_citation_clips_out_of_bounds_to_end() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 999,
        key: "K".into(),
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    assert_eq!(chars.len(), 4);
    assert_eq!(chars[3], CITATION_MARKER);
    let id = d.last_citation_id().unwrap();
    let c = d.citation(&id).unwrap();
    // Stored position is the CLIPPED `at`, not the raw 999.
    assert_eq!(c.at, 3);
}

#[test]
fn insert_citation_unique_ids_across_calls() {
    let mut d = Doc::new();
    d.insert(0, "abcdef");
    let mut ids = Vec::new();
    for _ in 0..5 {
        d.apply_edit_op(EditOp::InsertCitation {
            at: 0,
            key: "k".into(),
        })
        .unwrap();
        ids.push(d.last_citation_id().unwrap());
    }
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), 5);
}

#[test]
fn insert_citation_strictly_increasing_counter() {
    // Mutation-killer: catches the n+1 → n-1 mutation in next_id when
    // applied to the citation counter (a different counter key from
    // comment / suggestion).
    let mut d = Doc::new();
    let mut suffixes: Vec<i64> = Vec::new();
    for _ in 0..3 {
        d.apply_edit_op(EditOp::InsertCitation {
            at: 0,
            key: "k".into(),
        })
        .unwrap();
        let id = d.last_citation_id().unwrap();
        let parts: Vec<&str> = id.splitn(3, '-').collect();
        suffixes.push(parts[2].parse().unwrap());
    }
    assert!(suffixes[0] < suffixes[1] && suffixes[1] < suffixes[2]);
}

#[test]
fn insert_citation_preserves_existing_text() {
    let mut d = Doc::new();
    d.insert(0, "alpha beta");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 5,
        key: "K".into(),
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    // Marker between "alpha" and " beta".
    assert_eq!(&chars[..5].iter().collect::<String>(), "alpha");
    assert_eq!(chars[5], CITATION_MARKER);
    assert_eq!(&chars[6..].iter().collect::<String>(), " beta");
}

#[test]
fn insert_citation_lists_in_citation_ids() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 0,
        key: "k1".into(),
    })
    .unwrap();
    let id1 = d.last_citation_id().unwrap();
    d.apply_edit_op(EditOp::InsertCitation {
        at: 0,
        key: "k2".into(),
    })
    .unwrap();
    let id2 = d.last_citation_id().unwrap();
    let mut ids = d.citation_ids();
    ids.sort();
    let mut expected = vec![id1, id2];
    expected.sort();
    assert_eq!(ids, expected);
}

#[test]
fn citation_unknown_id_returns_none() {
    let d = Doc::new();
    assert!(d.citation("nope").is_none());
    assert!(d.citation_ids().is_empty());
}

#[test]
fn insert_citation_preserves_latam_keys() {
    let mut d = Doc::new();
    d.insert(0, "año");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 3,
        key: "Año2026".into(),
    })
    .unwrap();
    let id = d.last_citation_id().unwrap();
    let c = d.citation(&id).unwrap();
    assert_eq!(c.key, "Año2026");
}

// ────────────────────────────────────────────────────────────────
// Phase D — InsertFootnote variant
// ────────────────────────────────────────────────────────────────

#[test]
fn insert_footnote_inserts_marker_at_position() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 5,
        body: BlockTree {
            blocks: vec![paragraph("note body")],
        },
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    assert_eq!(chars[5], FOOTNOTE_MARKER);
    // The body text and footnote body live in DIFFERENT containers;
    // the body only carries the marker.
    assert_eq!(chars.len(), 6);
}

#[test]
fn insert_footnote_records_block_tree() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    let body = BlockTree {
        blocks: vec![
            paragraph("first para"),
            heading(2, "subhead"),
            paragraph("second para"),
        ],
    };
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 5,
        body: body.clone(),
    })
    .unwrap();
    let id = d.last_footnote_id().unwrap();
    let f = d.footnote(&id).expect("footnote present");
    assert_eq!(f.id, id);
    assert_eq!(f.at, 5);
    assert_eq!(f.body, body);
}

#[test]
fn insert_footnote_with_listitem_block_round_trips() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    let body = BlockTree {
        blocks: vec![list_item(2, "deeply nested item")],
    };
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: body.clone(),
    })
    .unwrap();
    let id = d.last_footnote_id().unwrap();
    let f = d.footnote(&id).unwrap();
    assert_eq!(f.body, body);
    assert_eq!(f.body.blocks[0].kind, BlockKind::ListItem { indent: 2 });
}

#[test]
fn insert_footnote_with_empty_block_tree_is_valid() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 1,
        body: BlockTree::default(),
    })
    .unwrap();
    let id = d.last_footnote_id().unwrap();
    let f = d.footnote(&id).unwrap();
    assert_eq!(f.body, BlockTree::default());
    assert_eq!(f.body.blocks.len(), 0);
}

#[test]
fn insert_footnote_at_end_appends_marker() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 3,
        body: BlockTree {
            blocks: vec![paragraph("note")],
        },
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    assert_eq!(chars.len(), 4);
    assert_eq!(chars[3], FOOTNOTE_MARKER);
}

#[test]
fn insert_footnote_clips_out_of_bounds() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 999,
        body: BlockTree::default(),
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    assert_eq!(chars.len(), 4);
    assert_eq!(chars[3], FOOTNOTE_MARKER);
    let id = d.last_footnote_id().unwrap();
    assert_eq!(d.footnote(&id).unwrap().at, 3);
}

#[test]
fn insert_footnote_unique_ids() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    let mut ids = Vec::new();
    for _ in 0..4 {
        d.apply_edit_op(EditOp::InsertFootnote {
            at: 0,
            body: BlockTree::default(),
        })
        .unwrap();
        ids.push(d.last_footnote_id().unwrap());
    }
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), 4);
}

#[test]
fn insert_footnote_strictly_increasing_counter() {
    let mut d = Doc::new();
    let mut suffixes: Vec<i64> = Vec::new();
    for _ in 0..3 {
        d.apply_edit_op(EditOp::InsertFootnote {
            at: 0,
            body: BlockTree::default(),
        })
        .unwrap();
        let id = d.last_footnote_id().unwrap();
        let parts: Vec<&str> = id.splitn(3, '-').collect();
        suffixes.push(parts[2].parse().unwrap());
    }
    assert!(suffixes[0] < suffixes[1] && suffixes[1] < suffixes[2]);
}

#[test]
fn footnote_unknown_id_returns_none() {
    let d = Doc::new();
    assert!(d.footnote("nope").is_none());
    assert!(d.footnote_ids().is_empty());
}

#[test]
fn insert_footnote_preserves_latam_text_in_blocks() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: BlockTree {
            blocks: vec![paragraph("ñoño año mañana día sí")],
        },
    })
    .unwrap();
    let id = d.last_footnote_id().unwrap();
    let f = d.footnote(&id).unwrap();
    assert_eq!(f.body.blocks[0].text, "ñoño año mañana día sí");
}

#[test]
fn citation_and_footnote_markers_distinct_in_body() {
    // The two markers are different codepoints so the layout engine
    // can tell them apart by reading the body alone.
    let mut d = Doc::new();
    d.insert(0, "AB");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 1,
        key: "K".into(),
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: BlockTree::default(),
    })
    .unwrap();
    let chars: Vec<char> = d.text().chars().collect();
    // Footnote was inserted at 0, then citation was at 1 of original, but
    // the citation insertion came FIRST so its position 1 was relative
    // to "AB"; after footnote prepends, citation marker shifts to index 2.
    //
    // Concretely: Step 1 inserts citation at pos 1 → "A<E000>B".
    // Step 2 inserts footnote at pos 0 → "<E001>A<E000>B".
    assert_eq!(chars.len(), 4);
    assert_eq!(chars[0], FOOTNOTE_MARKER);
    assert_eq!(chars[1], 'A');
    assert_eq!(chars[2], CITATION_MARKER);
    assert_eq!(chars[3], 'B');
}

// ────────────────────────────────────────────────────────────────
// Phase D — Snapshot durability
// ────────────────────────────────────────────────────────────────

#[test]
fn snapshot_round_trip_preserves_citations() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::InsertCitation {
        at: 5,
        key: "Smith2020".into(),
    })
    .unwrap();
    let id = d.last_citation_id().unwrap();
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    let c = restored.citation(&id).expect("survives");
    assert_eq!(c.at, 5);
    assert_eq!(c.key, "Smith2020");
    // Marker codepoint is also preserved in the body.
    let chars: Vec<char> = restored.text().chars().collect();
    assert_eq!(chars[5], CITATION_MARKER);
}

#[test]
fn snapshot_round_trip_preserves_footnotes_with_block_tree() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    let body = BlockTree {
        blocks: vec![
            heading(1, "Title"),
            paragraph("body para"),
            list_item(0, "list entry"),
        ],
    };
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 5,
        body: body.clone(),
    })
    .unwrap();
    let id = d.last_footnote_id().unwrap();
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    let f = restored.footnote(&id).unwrap();
    assert_eq!(f.body, body);
}

// ────────────────────────────────────────────────────────────────
// Phase D properties
// ────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// `citation_ids().len()` after N InsertCitation calls is exactly N.
    #[test]
    fn prop_citation_count_matches_inserts(n in 1usize..6) {
        let mut d = Doc::new();
        d.insert(0, "abc");
        for i in 0..n {
            d.apply_edit_op(EditOp::InsertCitation {
                at: 0, key: format!("k{i}"),
            }).unwrap();
        }
        prop_assert_eq!(d.citation_ids().len(), n);
    }

    /// Same for footnotes.
    #[test]
    fn prop_footnote_count_matches_inserts(n in 1usize..6) {
        let mut d = Doc::new();
        d.insert(0, "abc");
        for _ in 0..n {
            d.apply_edit_op(EditOp::InsertFootnote {
                at: 0, body: BlockTree::default(),
            }).unwrap();
        }
        prop_assert_eq!(d.footnote_ids().len(), n);
    }

    /// Inserting a footnote with arbitrary block content and reading
    /// it back yields the same `BlockTree` (modulo heading-level
    /// clamp, which is the expected encoding behavior).
    #[test]
    fn prop_footnote_body_round_trips(
        text1 in "[a-zñáé ]{0,20}",
        text2 in "[a-zñáé ]{0,20}",
        level in 1u8..=6,
    ) {
        let mut d = Doc::new();
        let body = BlockTree {
            blocks: vec![
                heading(level, &text1),
                paragraph(&text2),
            ],
        };
        d.apply_edit_op(EditOp::InsertFootnote {
            at: 0, body: body.clone(),
        }).unwrap();
        let id = d.last_footnote_id().unwrap();
        let f = d.footnote(&id).unwrap();
        prop_assert_eq!(f.body, body);
    }
}
