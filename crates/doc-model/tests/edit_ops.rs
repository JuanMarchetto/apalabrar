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
// Variants 4-11 — must return NotYetImplemented with the variant name
// ────────────────────────────────────────────────────────────────

fn paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph,
        text: text.into(),
    }
}

#[test]
fn insert_block_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: paragraph("hi"),
    });
    assert_eq!(result, Err(Error::NotYetImplemented("InsertBlock")));
}

#[test]
fn split_block_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::SplitBlock { at: 0 });
    assert_eq!(result, Err(Error::NotYetImplemented("SplitBlock")));
}

#[test]
fn merge_blocks_returns_not_yet_implemented() {
    let mut d = Doc::new();
    let result = d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 1,
    });
    assert_eq!(result, Err(Error::NotYetImplemented("MergeBlocks")));
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
// Stub variants must NOT mutate the document
// ────────────────────────────────────────────────────────────────

#[test]
fn deferred_variants_do_not_mutate_the_doc() {
    let mut d = Doc::new();
    d.insert(0, "untouched");
    let prior_text = d.text();

    // Fire each deferred variant; doc should remain bit-identical.
    let _ = d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: paragraph("ignored"),
    });
    let _ = d.apply_edit_op(EditOp::SplitBlock { at: 0 });
    let _ = d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 1,
    });
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

    /// `apply_edit_op` for any deferred variant returns the right
    /// `NotYetImplemented(name)` and leaves the doc untouched. The
    /// property covers each deferred variant; if a future Phase B/C
    /// implementation accidentally short-circuits one variant
    /// without updating the test, this property fails.
    #[test]
    fn prop_deferred_variants_preserve_text(
        initial in "[a-zA-Z ]{0,30}",
        which in 0usize..8,
    ) {
        let mut d = Doc::new();
        d.insert(0, &initial);
        let prior = d.text();
        let (op, name): (EditOp, &'static str) = match which {
            0 => (EditOp::InsertBlock { at: 0, block: paragraph("p") }, "InsertBlock"),
            1 => (EditOp::SplitBlock { at: 0 }, "SplitBlock"),
            2 => (EditOp::MergeBlocks { first: 0, second: 1 }, "MergeBlocks"),
            3 => (EditOp::InsertComment { from: 0, to: 1, body: "c".into(), thread_id: None }, "InsertComment"),
            4 => (EditOp::Suggest { from: 0, to: 1, replacement: "r".into() }, "Suggest"),
            5 => (EditOp::AcceptSuggestion { suggestion_id: "id".into() }, "AcceptSuggestion"),
            6 => (EditOp::InsertCitation { at: 0, key: "k".into() }, "InsertCitation"),
            _ => (EditOp::InsertFootnote { at: 0, body: BlockTree::default() }, "InsertFootnote"),
        };
        let result = d.apply_edit_op(op);
        prop_assert_eq!(result, Err(Error::NotYetImplemented(name)));
        prop_assert_eq!(d.text(), prior);
    }
}
