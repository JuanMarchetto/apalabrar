//! Phase 4.1 RED — integration tests against the doc-model `Doc`.
//!
//! These tests exercise the layout engine with documents built through
//! the same public API the editor will use at runtime: `Doc::new()`,
//! `Doc::insert`, `Doc::delete`, `apply_edit_op` for block-level ops,
//! plus snapshot round-trips. They verify that whatever the doc-model
//! exposes (block_count, kinds, text projection) the layout faithfully
//! reflects.

use apalabrar_doc_model::{Block, BlockKind, Doc, EditOp};
use apalabrar_layout::{LETTER_AT_96DPI, layout};

#[test]
fn fresh_doc_block_count_matches_layout_block_box_count() {
    let d = Doc::new();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn insert_text_does_not_split_blocks_without_newline() {
    let mut d = Doc::new();
    d.insert(0, "Hello world");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), 1);
}

#[test]
fn insert_newline_splits_into_two_blocks() {
    let mut d = Doc::new();
    d.insert(0, "Hello\nworld");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), 2);
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn insert_block_op_grows_block_count_in_layout() {
    let mut d = Doc::new();
    let before = d.block_count();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind: BlockKind::Heading { level: 2 },
            text: "Section".into(),
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(
        d.block_count() > before,
        "InsertBlock should grow block count"
    );
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn delete_range_reflects_in_layout() {
    let mut d = Doc::new();
    d.insert(0, "first\nsecond\nthird");
    assert_eq!(d.block_count(), 3);
    // Delete the second block boundary (the `\n` between second and third).
    // Range is over codepoints; "first\nsecond" is 12 chars, the \n at 12.
    d.delete(12..13);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), d.block_count());
    assert!(
        plan.block_box_count() <= 2,
        "deleting a `\\n` merges blocks"
    );
}

#[test]
fn snapshot_round_trip_preserves_layout() {
    let mut d = Doc::new();
    d.insert(0, "alpha\nbeta\ngamma");
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    let p1 = layout(&d, &LETTER_AT_96DPI).unwrap();
    let p2 = layout(&restored, &LETTER_AT_96DPI).unwrap();
    assert_eq!(p1, p2);
}

#[test]
fn split_block_op_adds_a_block_box() {
    let mut d = Doc::new();
    d.insert(0, "Hello world");
    let before = d.block_count();
    // Split at codepoint 5 — between "Hello" and " world".
    d.apply_edit_op(EditOp::SplitBlock { at: 5 }).unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(d.block_count() > before);
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn merge_blocks_op_removes_a_block_box() {
    let mut d = Doc::new();
    d.insert(0, "A\nB\nC");
    assert_eq!(d.block_count(), 3);
    let before = d.block_count();
    // Merge requires `first` and `second` to live in adjacent blocks.
    // For body "A\nB\nC": codepoint 0 is in block 0 (text "A"); codepoint
    // 2 is in block 1 (text "B"). Merging glues "A" and "B" into one
    // block, dropping block_count from 3 to 2.
    d.apply_edit_op(EditOp::MergeBlocks {
        first: 0,
        second: 2,
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(d.block_count() < before);
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn many_insert_text_ops_keep_invariant() {
    let mut d = Doc::new();
    for i in 0..50 {
        let s = format!("line {i}\n");
        let len = d.text().chars().count();
        d.insert(len, &s);
    }
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn doc_text_is_unchanged_by_layout() {
    let mut d = Doc::new();
    d.insert(0, "alpha\nbeta\ngamma");
    let before = d.text();
    let _ = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(d.text(), before);
}
