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

#[test]
fn insert_comment_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 1,
        body: "todo".into(),
        thread_id: None,
    });
    assert_eq!(result, Err(Error::NotYetImplemented("InsertComment")));
}

#[test]
fn suggest_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 1,
        replacement: "x".into(),
    });
    assert_eq!(result, Err(Error::NotYetImplemented("Suggest")));
}

#[test]
fn accept_suggestion_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: "abc".into(),
    });
    assert_eq!(result, Err(Error::NotYetImplemented("AcceptSuggestion")));
}

#[test]
fn insert_citation_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::InsertCitation {
        at: 0,
        key: "Smith2020".into(),
    });
    assert_eq!(result, Err(Error::NotYetImplemented("InsertCitation")));
}

#[test]
fn insert_footnote_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: BlockTree {
            blocks: vec![paragraph("note")],
        },
    });
    assert_eq!(result, Err(Error::NotYetImplemented("InsertFootnote")));
}

// ────────────────────────────────────────────────────────────────
// Stub variants must NOT mutate the document text or marks
// (InsertBlock / SplitBlock / MergeBlocks were promoted to
// implemented in Phase B and have their own tests below.)
// ────────────────────────────────────────────────────────────────

#[test]
fn deferred_variants_do_not_mutate_the_doc() {
    let mut d = Doc::new();
    d.insert(0, "untouched");
    let prior_text = d.text();

    // Fire each STILL-deferred variant; doc should remain bit-identical.
    let _ = d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 1,
        body: "x".into(),
        thread_id: None,
    });
    let _ = d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 1,
        replacement: "y".into(),
    });
    let _ = d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: "z".into(),
    });
    let _ = d.apply_edit_op(EditOp::InsertCitation {
        at: 0,
        key: "k".into(),
    });
    let _ = d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: BlockTree::default(),
    });

    assert_eq!(d.text(), prior_text);
    // No marks should have been applied either.
    assert!(!d.has_mark(0, Mark::Bold));
    assert!(!d.has_mark(0, Mark::Italic));
}

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

    /// `apply_edit_op` for any STILL-deferred variant returns the
    /// right `NotYetImplemented(name)` and leaves the doc untouched.
    /// Phase B promoted InsertBlock/SplitBlock/MergeBlocks to
    /// implemented, so they're no longer covered here.
    #[test]
    fn prop_deferred_variants_preserve_text(
        initial in "[a-zA-Z ]{0,30}",
        which in 0usize..5,
    ) {
        let mut d = Doc::new();
        d.insert(0, &initial);
        let prior = d.text();
        let (op, name): (EditOp, &'static str) = match which {
            0 => (EditOp::InsertComment { from: 0, to: 1, body: "c".into(), thread_id: None }, "InsertComment"),
            1 => (EditOp::Suggest { from: 0, to: 1, replacement: "r".into() }, "Suggest"),
            2 => (EditOp::AcceptSuggestion { suggestion_id: "id".into() }, "AcceptSuggestion"),
            3 => (EditOp::InsertCitation { at: 0, key: "k".into() }, "InsertCitation"),
            _ => (EditOp::InsertFootnote { at: 0, body: BlockTree::default() }, "InsertFootnote"),
        };
        let result = d.apply_edit_op(op);
        prop_assert_eq!(result, Err(Error::NotYetImplemented(name)));
        prop_assert_eq!(d.text(), prior);
    }
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
