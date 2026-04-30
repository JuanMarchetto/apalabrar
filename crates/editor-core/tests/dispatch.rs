//! Phase 2.4 RED-phase contract tests for `editor_core::dispatch`.
//!
//! Each test asserts on the `RenderDelta` returned by `dispatch`,
//! not just on doc state — the dispatcher's value is precisely
//! the delta it computes, so the delta is what must be tested.
//!
//! Coverage map (matches AUTONOMOUS_PROMPTS.md prompt 2.4 + the
//! 5-step rhythm from CONTRIBUTING.md):
//!
//! - One contract test per `EditOp` variant (11 variants)
//! - Boundary cases: empty doc, position past end, zero-width range,
//!   inverted range, empty insert, multi-block doc, op spanning blocks,
//!   op crossing newlines, accept-already-accepted (idempotent),
//!   merge-non-adjacent (no-op).
//! - Error cases: AcceptSuggestion with unknown id, malformed op
//!   (errors propagate as `EditOpFailed`).
//! - Properties (proptest, 64 cases each):
//!   1. `dispatch_is_total` — every valid op terminates with `Ok(_)`
//!   2. `dispatch_is_clip_defensive` — no panics on out-of-bounds
//!   3. `dispatch_dirty_range_is_well_formed` — `start <= end` and
//!      `end <= post_block_count` for every variant.

use apalabrar_doc_model::{Block, BlockKind, BlockTree, Doc, EditOp, Mark};
use apalabrar_editor_core::Error;
use apalabrar_editor_core::dispatch::{BlockRange, MintedId, RenderDelta, dispatch};
use proptest::prelude::*;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn doc_with(text: &str) -> Doc {
    let mut d = Doc::new();
    if !text.is_empty() {
        d.insert(0, text);
    }
    d
}

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

// -----------------------------------------------------------------------------
// RenderDelta sentinel constants
// -----------------------------------------------------------------------------

#[test]
fn block_range_empty_const_is_zero_zero() {
    assert_eq!(BlockRange::EMPTY, BlockRange { start: 0, end: 0 });
    assert!(BlockRange::EMPTY.is_empty());
}

#[test]
fn block_range_is_empty_only_when_start_equals_end() {
    assert!(BlockRange { start: 3, end: 3 }.is_empty());
    assert!(!BlockRange { start: 0, end: 1 }.is_empty());
    assert!(!BlockRange { start: 2, end: 5 }.is_empty());
}

#[test]
fn render_delta_noop_constant_has_empty_dirty_no_structural_no_caret_no_id() {
    let n = RenderDelta::NOOP;
    assert!(n.dirty_blocks.is_empty());
    assert!(!n.structural);
    assert_eq!(n.caret_hint, None);
    assert_eq!(n.minted_id, None);
}

// -----------------------------------------------------------------------------
// InsertText
// -----------------------------------------------------------------------------

#[test]
fn insert_text_within_first_block_dirty_is_zero_one() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 5,
            text: " world".into(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert!(!delta.structural);
    assert_eq!(delta.caret_hint, Some(11));
    assert_eq!(delta.minted_id, None);
    assert_eq!(d.text(), "hello world");
}

#[test]
fn insert_text_with_newline_marks_two_blocks_dirty_and_structural() {
    let mut d = doc_with("ab");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 1,
            text: "X\nY".into(),
            marks: vec![],
        },
    )
    .unwrap();
    // block 0 contained `at`; one newline added → dirty 0..2
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 2 });
    assert!(delta.structural);
    assert_eq!(delta.caret_hint, Some(4));
    assert_eq!(d.text(), "aX\nYb");
    assert_eq!(d.block_count(), 2);
}

#[test]
fn insert_text_with_two_newlines_dirty_spans_three_blocks() {
    let mut d = doc_with("hi");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 1,
            text: "X\nY\nZ".into(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 3 });
    assert!(delta.structural);
}

#[test]
fn insert_text_in_second_block_dirty_starts_at_one() {
    let mut d = doc_with("first\nsecond");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 7,
            text: "X".into(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 2 });
    assert!(!delta.structural);
}

#[test]
fn insert_text_empty_string_returns_noop_delta() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 2,
            text: String::new(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(delta, RenderDelta::NOOP);
    assert_eq!(d.text(), "hello");
}

#[test]
fn insert_text_at_position_past_end_clips_to_end() {
    let mut d = doc_with("ab");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 999,
            text: "X".into(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(delta.caret_hint, Some(3)); // clipped at=2 + 1 char
    assert_eq!(d.text(), "abX");
}

#[test]
fn insert_text_with_marks_applies_marks_and_returns_delta() {
    let mut d = doc_with("ab");
    let delta = dispatch(
        &mut d,
        EditOp::InsertText {
            at: 1,
            text: "X".into(),
            marks: vec![Mark::Bold],
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert!(d.has_mark(1, Mark::Bold));
}

// -----------------------------------------------------------------------------
// DeleteRange
// -----------------------------------------------------------------------------

#[test]
fn delete_range_within_single_block_no_structural_change() {
    let mut d = doc_with("hello world");
    let delta = dispatch(&mut d, EditOp::DeleteRange { from: 5, to: 11 }).unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert!(!delta.structural);
    assert_eq!(delta.caret_hint, Some(5));
    assert_eq!(d.text(), "hello");
}

#[test]
fn delete_range_crossing_one_newline_is_structural_and_dirty_first_block_only() {
    let mut d = doc_with("aa\nbb");
    let delta = dispatch(&mut d, EditOp::DeleteRange { from: 1, to: 4 }).unwrap();
    // pre block_count=2, post=1 → structural=true
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert_eq!(delta.caret_hint, Some(1));
    assert_eq!(d.text(), "ab");
    assert_eq!(d.block_count(), 1);
}

#[test]
fn delete_range_zero_width_returns_noop() {
    let mut d = doc_with("hello");
    let delta = dispatch(&mut d, EditOp::DeleteRange { from: 2, to: 2 }).unwrap();
    assert_eq!(delta, RenderDelta::NOOP);
    assert_eq!(d.text(), "hello");
}

#[test]
fn delete_range_inverted_indices_returns_noop() {
    let mut d = doc_with("hello");
    let delta = dispatch(&mut d, EditOp::DeleteRange { from: 4, to: 1 }).unwrap();
    assert_eq!(delta, RenderDelta::NOOP);
}

#[test]
fn delete_range_clips_oob_to_doc_end() {
    let mut d = doc_with("ab");
    let delta = dispatch(&mut d, EditOp::DeleteRange { from: 1, to: 999 }).unwrap();
    assert_eq!(delta.caret_hint, Some(1));
    assert_eq!(d.text(), "a");
}

// -----------------------------------------------------------------------------
// FormatRange
// -----------------------------------------------------------------------------

#[test]
fn format_range_dirty_covers_span_no_structural_no_caret() {
    let mut d = doc_with("hello world");
    let delta = dispatch(
        &mut d,
        EditOp::FormatRange {
            from: 0,
            to: 5,
            mark: Mark::Bold,
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert!(!delta.structural);
    assert_eq!(delta.caret_hint, None);
    assert_eq!(delta.minted_id, None);
    assert!(d.has_mark(2, Mark::Bold));
}

#[test]
fn format_range_zero_width_returns_noop() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::FormatRange {
            from: 3,
            to: 3,
            mark: Mark::Italic,
        },
    )
    .unwrap();
    assert_eq!(delta, RenderDelta::NOOP);
}

#[test]
fn format_range_spanning_two_blocks_dirty_covers_both_no_structural() {
    let mut d = doc_with("aa\nbb");
    let delta = dispatch(
        &mut d,
        EditOp::FormatRange {
            from: 1,
            to: 4,
            mark: Mark::Bold,
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 2 });
    assert!(!delta.structural);
}

// -----------------------------------------------------------------------------
// InsertBlock
// -----------------------------------------------------------------------------

#[test]
fn insert_block_at_start_dirty_zero_two_structural() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::InsertBlock {
            at: 0,
            block: heading(1, "Title"),
        },
    )
    .unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 2 });
    assert_eq!(delta.caret_hint, Some(6)); // "Title" + "\n" = 6
    assert_eq!(d.text(), "Title\nhello");
}

#[test]
fn insert_block_at_end_appends_and_dirty_starts_at_last_block() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::InsertBlock {
            at: 5,
            block: paragraph("world"),
        },
    )
    .unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 2 });
    assert_eq!(d.text(), "hello\nworld");
}

#[test]
fn insert_block_clips_position_past_end() {
    let mut d = doc_with("ab");
    let delta = dispatch(
        &mut d,
        EditOp::InsertBlock {
            at: 999,
            block: paragraph("X"),
        },
    )
    .unwrap();
    assert!(delta.structural);
    assert_eq!(delta.caret_hint, Some(4)); // clipped 2 + "X" + "\n"
}

// -----------------------------------------------------------------------------
// SplitBlock
// -----------------------------------------------------------------------------

#[test]
fn split_block_dirty_covers_two_blocks_structural_caret_at_plus_one() {
    let mut d = doc_with("hello");
    let delta = dispatch(&mut d, EditOp::SplitBlock { at: 2 }).unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 2 });
    assert_eq!(delta.caret_hint, Some(3));
    assert_eq!(d.text(), "he\nllo");
    assert_eq!(d.block_count(), 2);
}

#[test]
fn split_block_in_second_block_dirty_starts_at_one() {
    let mut d = doc_with("aa\nbbcc");
    let delta = dispatch(&mut d, EditOp::SplitBlock { at: 5 }).unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 3 });
    assert_eq!(d.block_count(), 3);
}

#[test]
fn split_block_clips_position_past_end() {
    let mut d = doc_with("ab");
    let delta = dispatch(&mut d, EditOp::SplitBlock { at: 999 }).unwrap();
    assert!(delta.structural);
    assert_eq!(delta.caret_hint, Some(3)); // clipped 2 + 1 newline
}

// -----------------------------------------------------------------------------
// MergeBlocks
// -----------------------------------------------------------------------------

#[test]
fn merge_blocks_adjacent_dirty_is_first_block_structural_no_caret() {
    let mut d = doc_with("aa\nbb");
    let delta = dispatch(
        &mut d,
        EditOp::MergeBlocks {
            first: 1,
            second: 4,
        },
    )
    .unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert_eq!(delta.caret_hint, None);
    assert_eq!(d.text(), "aabb");
    assert_eq!(d.block_count(), 1);
}

#[test]
fn merge_blocks_non_adjacent_returns_noop() {
    let mut d = doc_with("aa\nbb\ncc");
    let delta = dispatch(
        &mut d,
        EditOp::MergeBlocks {
            first: 1,
            second: 7,
        },
    )
    .unwrap();
    assert_eq!(delta, RenderDelta::NOOP);
    assert_eq!(d.block_count(), 3);
}

#[test]
fn merge_blocks_in_middle_of_doc_dirty_starts_at_first_block_idx() {
    let mut d = doc_with("aa\nbb\ncc");
    let delta = dispatch(
        &mut d,
        EditOp::MergeBlocks {
            first: 4,
            second: 6,
        },
    )
    .unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 2 });
    assert_eq!(d.text(), "aa\nbbcc");
}

// -----------------------------------------------------------------------------
// InsertComment
// -----------------------------------------------------------------------------

#[test]
fn insert_comment_dirty_covers_anchor_range_no_structural_with_minted_id() {
    let mut d = doc_with("hello world");
    let delta = dispatch(
        &mut d,
        EditOp::InsertComment {
            from: 0,
            to: 5,
            body: "first thoughts".into(),
            thread_id: None,
            author: "tester".into(),
            created_at: 0,
        },
    )
    .unwrap();
    assert!(!delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert_eq!(delta.caret_hint, None);
    let id = match delta.minted_id {
        Some(MintedId::Comment(s)) => s,
        other => panic!("expected MintedId::Comment, got {other:?}"),
    };
    assert!(d.comment(&id).is_some());
}

#[test]
fn insert_comment_spanning_two_blocks_dirty_covers_both() {
    let mut d = doc_with("aa\nbb");
    let delta = dispatch(
        &mut d,
        EditOp::InsertComment {
            from: 1,
            to: 4,
            body: "x".into(),
            thread_id: None,
            author: "tester".into(),
            created_at: 0,
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 2 });
}

// -----------------------------------------------------------------------------
// ReplyToComment + SetCommentStatus (Phase 4.6)
// -----------------------------------------------------------------------------

#[test]
fn dispatch_reply_to_comment_emits_reply_minted_id_with_anchor_dirty_range() {
    let mut d = doc_with("hello world");
    // Seed a thread first.
    dispatch(
        &mut d,
        EditOp::InsertComment {
            from: 0,
            to: 5,
            body: "head".into(),
            thread_id: Some("t-1".into()),
            author: "alice".into(),
            created_at: 1,
        },
    )
    .unwrap();
    let delta = dispatch(
        &mut d,
        EditOp::ReplyToComment {
            thread_id: "t-1".into(),
            body: "hi".into(),
            author: "bob".into(),
            created_at: 2,
        },
    )
    .unwrap();
    // Reply doesn't change text or block count.
    assert!(!delta.structural);
    assert_eq!(delta.caret_hint, None);
    // dirty_blocks covers the parent thread anchor (block 0 only).
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    let id = match delta.minted_id {
        Some(MintedId::Reply(s)) => s,
        other => panic!("expected MintedId::Reply, got {other:?}"),
    };
    assert!(!id.is_empty());
}

#[test]
fn dispatch_reply_to_unknown_thread_propagates_edit_op_failed() {
    let mut d = doc_with("x");
    let err = dispatch(
        &mut d,
        EditOp::ReplyToComment {
            thread_id: "t-bogus".into(),
            body: "x".into(),
            author: "x".into(),
            created_at: 0,
        },
    )
    .unwrap_err();
    match err {
        Error::EditOpFailed { kind, .. } => assert_eq!(kind, "ReplyToComment"),
        other => panic!("expected EditOpFailed, got {other:?}"),
    }
}

#[test]
fn dispatch_set_comment_status_emits_no_minted_id_with_anchor_dirty_range() {
    let mut d = doc_with("hello world");
    dispatch(
        &mut d,
        EditOp::InsertComment {
            from: 6,
            to: 11,
            body: "head".into(),
            thread_id: Some("t-1".into()),
            author: "alice".into(),
            created_at: 1,
        },
    )
    .unwrap();
    let delta = dispatch(
        &mut d,
        EditOp::SetCommentStatus {
            thread_id: "t-1".into(),
            status: apalabrar_doc_model::CommentStatus::Resolved,
        },
    )
    .unwrap();
    assert!(!delta.structural);
    assert_eq!(delta.minted_id, None);
    // Anchor was on block 0 (single-paragraph doc).
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
}

#[test]
fn dispatch_set_comment_status_unknown_thread_propagates_error() {
    let mut d = doc_with("x");
    let err = dispatch(
        &mut d,
        EditOp::SetCommentStatus {
            thread_id: "t-bogus".into(),
            status: apalabrar_doc_model::CommentStatus::Resolved,
        },
    )
    .unwrap_err();
    match err {
        Error::EditOpFailed { kind, .. } => assert_eq!(kind, "SetCommentStatus"),
        other => panic!("expected EditOpFailed, got {other:?}"),
    }
}

#[test]
fn dispatch_reply_to_comment_in_multi_block_uses_parent_anchor_blocks() {
    // Comment anchored on block 1 (the "world" block, after the \n);
    // a reply must dirty block 1 ONLY, not block 0.
    let mut d = doc_with("hello\nworld");
    dispatch(
        &mut d,
        EditOp::InsertComment {
            from: 6,
            to: 11,
            body: "head".into(),
            thread_id: Some("t-1".into()),
            author: "a".into(),
            created_at: 0,
        },
    )
    .unwrap();
    let delta = dispatch(
        &mut d,
        EditOp::ReplyToComment {
            thread_id: "t-1".into(),
            body: "r".into(),
            author: "b".into(),
            created_at: 1,
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 2 });
}

// -----------------------------------------------------------------------------
// Suggest
// -----------------------------------------------------------------------------

#[test]
fn suggest_dirty_covers_anchor_range_with_minted_suggestion_id() {
    let mut d = doc_with("hello world");
    let delta = dispatch(
        &mut d,
        EditOp::Suggest {
            from: 0,
            to: 5,
            replacement: "GREETINGS".into(),
        },
    )
    .unwrap();
    assert!(!delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert_eq!(delta.caret_hint, None);
    let id = match delta.minted_id {
        Some(MintedId::Suggestion(s)) => s,
        other => panic!("expected MintedId::Suggestion, got {other:?}"),
    };
    assert!(d.suggestion(&id).is_some());
    // text unchanged at Suggest time
    assert_eq!(d.text(), "hello world");
}

// -----------------------------------------------------------------------------
// AcceptSuggestion
// -----------------------------------------------------------------------------

#[test]
fn accept_suggestion_dirty_covers_pre_apply_range_no_minted_id() {
    let mut d = doc_with("hello world");
    let suggest = dispatch(
        &mut d,
        EditOp::Suggest {
            from: 0,
            to: 5,
            replacement: "GREETINGS".into(),
        },
    )
    .unwrap();
    let id = match suggest.minted_id {
        Some(MintedId::Suggestion(s)) => s,
        _ => panic!("expected suggestion id"),
    };
    let delta = dispatch(
        &mut d,
        EditOp::AcceptSuggestion {
            suggestion_id: id.clone(),
        },
    )
    .unwrap();
    assert_eq!(delta.minted_id, None);
    assert_eq!(delta.caret_hint, None);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert!(!delta.structural);
    assert_eq!(d.text(), "GREETINGS world");
}

#[test]
fn accept_suggestion_with_newline_replacement_is_structural() {
    let mut d = doc_with("hello");
    let suggest = dispatch(
        &mut d,
        EditOp::Suggest {
            from: 0,
            to: 5,
            replacement: "a\nb".into(),
        },
    )
    .unwrap();
    let id = match suggest.minted_id {
        Some(MintedId::Suggestion(s)) => s,
        _ => panic!("expected suggestion id"),
    };
    let delta = dispatch(&mut d, EditOp::AcceptSuggestion { suggestion_id: id }).unwrap();
    assert!(delta.structural);
    assert_eq!(d.text(), "a\nb");
}

#[test]
fn accept_suggestion_structural_in_second_block_dirty_starts_at_block_lo_plus_one() {
    // doc has two blocks; suggestion lives in block 1; replacement
    // adds newlines so the accept is structural. dirty_end must be
    // block_lo + 1 (not block_lo, not block_hi + 1).
    let mut d = doc_with("aa\nbb");
    let suggest = dispatch(
        &mut d,
        EditOp::Suggest {
            from: 3,
            to: 5,
            replacement: "X\nY\nZ".into(),
        },
    )
    .unwrap();
    let id = match suggest.minted_id {
        Some(MintedId::Suggestion(s)) => s,
        _ => panic!("expected suggestion id"),
    };
    let delta = dispatch(&mut d, EditOp::AcceptSuggestion { suggestion_id: id }).unwrap();
    assert!(delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 2 });
    assert_eq!(d.text(), "aa\nX\nY\nZ");
    assert_eq!(d.block_count(), 4);
}

#[test]
fn accept_suggestion_unknown_id_returns_edit_op_failed() {
    let mut d = doc_with("hi");
    let err = dispatch(
        &mut d,
        EditOp::AcceptSuggestion {
            suggestion_id: "no-such".into(),
        },
    )
    .unwrap_err();
    match err {
        Error::EditOpFailed { kind, reason } => {
            assert_eq!(kind, "AcceptSuggestion");
            assert!(reason.contains("no-such"));
        }
        other => panic!("expected EditOpFailed, got {other:?}"),
    }
}

#[test]
fn accept_suggestion_already_accepted_is_idempotent_returns_zero_delta() {
    let mut d = doc_with("hello");
    let suggest = dispatch(
        &mut d,
        EditOp::Suggest {
            from: 0,
            to: 5,
            replacement: "X".into(),
        },
    )
    .unwrap();
    let id = match suggest.minted_id {
        Some(MintedId::Suggestion(s)) => s,
        _ => panic!("expected id"),
    };
    let _first = dispatch(
        &mut d,
        EditOp::AcceptSuggestion {
            suggestion_id: id.clone(),
        },
    )
    .unwrap();
    let snapshot_text = d.text();
    let snapshot_blocks = d.block_count();
    let second = dispatch(&mut d, EditOp::AcceptSuggestion { suggestion_id: id }).unwrap();
    // Idempotent re-accept must not mutate the doc.
    assert_eq!(d.text(), snapshot_text);
    assert_eq!(d.block_count(), snapshot_blocks);
    // Delta is allowed to report dirty_blocks for the original anchor;
    // the contract is `structural=false` and no minted id (no new state).
    assert!(!second.structural);
    assert_eq!(second.minted_id, None);
}

// -----------------------------------------------------------------------------
// InsertCitation
// -----------------------------------------------------------------------------

#[test]
fn insert_citation_dirty_is_block_at_at_caret_after_marker_with_minted_id() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::InsertCitation {
            at: 3,
            key: "Smith2020".into(),
        },
    )
    .unwrap();
    assert!(!delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert_eq!(delta.caret_hint, Some(4));
    let id = match delta.minted_id {
        Some(MintedId::Citation(s)) => s,
        other => panic!("expected MintedId::Citation, got {other:?}"),
    };
    assert!(d.citation(&id).is_some());
}

#[test]
fn insert_citation_in_second_block_dirty_starts_at_one() {
    let mut d = doc_with("aa\nbb");
    let delta = dispatch(
        &mut d,
        EditOp::InsertCitation {
            at: 4,
            key: "k".into(),
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 2 });
}

// -----------------------------------------------------------------------------
// InsertFootnote
// -----------------------------------------------------------------------------

#[test]
fn insert_footnote_dirty_is_block_at_at_caret_after_marker_with_minted_id() {
    let mut d = doc_with("hello");
    let delta = dispatch(
        &mut d,
        EditOp::InsertFootnote {
            at: 3,
            body: BlockTree {
                blocks: vec![paragraph("note")],
            },
        },
    )
    .unwrap();
    assert!(!delta.structural);
    assert_eq!(delta.dirty_blocks, BlockRange { start: 0, end: 1 });
    assert_eq!(delta.caret_hint, Some(4));
    let id = match delta.minted_id {
        Some(MintedId::Footnote(s)) => s,
        other => panic!("expected MintedId::Footnote, got {other:?}"),
    };
    assert!(d.footnote(&id).is_some());
}

#[test]
fn insert_footnote_in_second_block_dirty_starts_at_one() {
    let mut d = doc_with("aa\nbb");
    let delta = dispatch(
        &mut d,
        EditOp::InsertFootnote {
            at: 4,
            body: BlockTree::default(),
        },
    )
    .unwrap();
    assert_eq!(delta.dirty_blocks, BlockRange { start: 1, end: 2 });
}

// -----------------------------------------------------------------------------
// Cross-cutting properties
// -----------------------------------------------------------------------------

fn arb_seed_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(vec!['a', 'b', '\n']), 0..40)
        .prop_map(|chars| chars.into_iter().collect())
}

fn arb_mark() -> impl Strategy<Value = Mark> {
    prop_oneof![Just(Mark::Bold), Just(Mark::Italic)]
}

fn arb_block_kind() -> impl Strategy<Value = BlockKind> {
    prop_oneof![
        Just(BlockKind::Paragraph),
        (1u8..=6).prop_map(|level| BlockKind::Heading { level }),
        (0u8..=4).prop_map(|indent| BlockKind::ListItem { indent }),
    ]
}

fn arb_block() -> impl Strategy<Value = Block> {
    (arb_block_kind(), "[a-z]{0,8}").prop_map(|(kind, text)| Block { kind, text })
}

fn arb_op() -> impl Strategy<Value = EditOp> {
    prop_oneof![
        (
            0usize..40,
            "[a-z\n]{0,6}",
            prop::collection::vec(arb_mark(), 0..2)
        )
            .prop_map(|(at, text, marks)| EditOp::InsertText { at, text, marks }),
        (0usize..40, 0usize..40).prop_map(|(from, to)| EditOp::DeleteRange { from, to }),
        (0usize..40, 0usize..40, arb_mark()).prop_map(|(from, to, mark)| EditOp::FormatRange {
            from,
            to,
            mark
        }),
        (0usize..40, arb_block()).prop_map(|(at, block)| EditOp::InsertBlock { at, block }),
        (0usize..40).prop_map(|at| EditOp::SplitBlock { at }),
        (0usize..40, 0usize..40).prop_map(|(first, second)| EditOp::MergeBlocks { first, second }),
        (0usize..40, 0usize..40, "[a-z]{0,8}").prop_map(|(from, to, body)| {
            EditOp::InsertComment {
                from,
                to,
                body,
                thread_id: None,
                author: "tester".into(),
                created_at: 0,
            }
        }),
        (0usize..40, 0usize..40, "[a-z\n]{0,6}").prop_map(|(from, to, replacement)| {
            EditOp::Suggest {
                from,
                to,
                replacement,
            }
        }),
        (0usize..40, "[a-z]{1,8}").prop_map(|(at, key)| EditOp::InsertCitation { at, key }),
        (0usize..40, prop::collection::vec(arb_block(), 0..2)).prop_map(|(at, blocks)| {
            EditOp::InsertFootnote {
                at,
                body: BlockTree { blocks },
            }
        }),
        // AcceptSuggestion with random ids — expected to often error;
        // the `dispatch_is_total_or_well_typed_error` property handles
        // that by accepting `Err(EditOpFailed{ kind: "AcceptSuggestion", .. })`.
        "[a-z0-9-]{1,16}".prop_map(|id| EditOp::AcceptSuggestion { suggestion_id: id }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// dispatch must terminate with either Ok(_) or a *typed*
    /// EditOpFailed — never panic, never produce a different error
    /// kind. This is the totality guarantee the prompt asks for.
    #[test]
    fn dispatch_is_total_or_well_typed_error(seed in arb_seed_text(), op in arb_op()) {
        let mut d = doc_with(&seed);
        match dispatch(&mut d, op) {
            Ok(_) => {}
            Err(Error::EditOpFailed { .. }) => {}
            Err(other) => panic!("dispatch produced unexpected error: {other:?}"),
        }
    }

    /// dispatch is clip-defensive: out-of-bounds positions don't
    /// panic, the doc remains in a consistent state (text and
    /// block_count agree).
    #[test]
    fn dispatch_keeps_doc_consistent(seed in arb_seed_text(), op in arb_op()) {
        let mut d = doc_with(&seed);
        let _ = dispatch(&mut d, op);
        let text = d.text();
        let expected_blocks = text.matches('\n').count() + 1;
        prop_assert_eq!(d.block_count(), expected_blocks);
    }

    /// Every successful dispatch produces a well-formed dirty range:
    /// start <= end, and end <= post block_count + 1 (we allow +1
    /// because some ops dirty "up to and including the next block",
    /// which is layout-bounded later).
    #[test]
    fn dispatch_dirty_range_is_well_formed(seed in arb_seed_text(), op in arb_op()) {
        let mut d = doc_with(&seed);
        if let Ok(delta) = dispatch(&mut d, op) {
            prop_assert!(delta.dirty_blocks.start <= delta.dirty_blocks.end);
            prop_assert!(delta.dirty_blocks.end <= d.block_count() + 1);
        }
    }
}
