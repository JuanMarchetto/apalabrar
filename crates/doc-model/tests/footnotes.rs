//! Phase 5.1 RED — footnote anchoring (marks-based) + navigation.
//!
//! Locked semantics:
//! - Body insertion of `\u{E001}` is mark-anchored: a Loro mark with
//!   key "footnote" and value = footnote_id covers EXACTLY the marker
//!   codepoint. The mark is the authoritative anchor; the stored `at`
//!   on the snapshot is historical only.
//! - `find_footnote_range(id)` returns `Some(start..end)` with
//!   `end - start == 1`, or `None` if the mark is absent (unknown id /
//!   pre-5.1 snapshot).
//! - `footnote_at(pos)` returns `Some(id)` when `pos` lies on the
//!   marker codepoint; `None` otherwise.
//! - `footnotes_in_body_order()` returns `Vec<Footnote>` sorted by the
//!   CURRENT marker position (not by id), so display numbering matches
//!   reading order even when footnotes are inserted out of order.

use apalabrar_doc_model::{Block, BlockKind, BlockTree, Doc, EditOp, Footnote};
use proptest::prelude::*;

fn paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph,
        text: text.into(),
    }
}

fn doc_with_one_footnote(at: usize, body_text: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::InsertFootnote {
        at,
        body: BlockTree {
            blocks: vec![paragraph(body_text)],
        },
    })
    .unwrap();
    d
}

// ─────────────────────────────────────────────────────────────────
// Marks anchoring — find_footnote_range / footnote_at
// ─────────────────────────────────────────────────────────────────

#[test]
fn find_footnote_range_returns_single_codepoint_range() {
    let d = doc_with_one_footnote(5, "annotation");
    let id = d.last_footnote_id().unwrap();
    let r = d.find_footnote_range(&id).expect("mark must exist");
    // Marker is one codepoint wide.
    assert_eq!(r.end - r.start, 1);
    assert_eq!(r.start, 5);
}

#[test]
fn find_footnote_range_returns_none_for_unknown_id() {
    let d = Doc::new();
    assert_eq!(d.find_footnote_range("fn-bogus"), None);
}

#[test]
fn find_footnote_range_returns_none_when_no_footnotes_exist_in_doc_with_text() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    assert_eq!(d.find_footnote_range("fn1"), None);
}

#[test]
fn footnote_at_returns_id_at_marker_position() {
    let d = doc_with_one_footnote(5, "x");
    let id = d.last_footnote_id().unwrap();
    // The marker was inserted AT position 5, so position 5 IS the
    // marker codepoint.
    assert_eq!(d.footnote_at(5), Some(id));
}

#[test]
fn footnote_at_returns_none_at_position_before_marker() {
    let d = doc_with_one_footnote(5, "x");
    // Position 4 is the last char of "hello", not the marker.
    assert_eq!(d.footnote_at(4), None);
}

#[test]
fn footnote_at_returns_none_at_position_well_past_marker() {
    let d = doc_with_one_footnote(5, "x");
    // Position 10 is inside "world" (the body tail past the marker),
    // not the marker. Body chars are: hello\u{E001} world → 12 chars
    // total; pos 10 = 'r' in "world".
    assert_eq!(d.footnote_at(10), None);
}

#[test]
fn footnote_at_returns_none_for_position_exactly_at_end_of_marker() {
    // Mutation guard: the boundary check inside `footnote_at` must be
    // `pos < cursor + chars`, not `<=`. Mark covers [5..6) — pos=6 is
    // the FIRST char of the next segment, NOT part of the mark.
    let d = doc_with_one_footnote(5, "x");
    assert_eq!(d.footnote_at(6), None);
}

#[test]
fn footnote_at_finds_marker_on_a_non_first_segment() {
    // Mutation guard: the cursor advance `cursor += chars` must be
    // additive. Insert text BEFORE the marker so the loop has to
    // advance past segment 0 to find the mark.
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 11,
        body: BlockTree {
            blocks: vec![paragraph("note")],
        },
    })
    .unwrap();
    let id = d.last_footnote_id().unwrap();
    // Marker is the LAST codepoint (pos 11). After to_delta the
    // unmarked "hello world" segment must be consumed before the
    // marked marker segment — additive cursor advance is required.
    assert_eq!(d.footnote_at(11), Some(id));
}

#[test]
fn find_footnote_range_tracks_marker_after_text_inserted_before() {
    // Critical anchor-stability test: insert text BEFORE the marker;
    // the mark (and thus find_footnote_range) must follow the marker
    // to its new position.
    let mut d = doc_with_one_footnote(5, "annot");
    let id = d.last_footnote_id().unwrap();
    // Insert "X" at codepoint 0 — marker shifts from 5 → 6.
    d.insert(0, "X");
    let r = d
        .find_footnote_range(&id)
        .expect("mark must follow marker codepoint");
    assert_eq!(r.start, 6);
    assert_eq!(r.end - r.start, 1);
}

#[test]
fn find_footnote_range_unaffected_by_text_inserted_after_marker() {
    // Mark anchors to its codepoint, not to the codepoint *after* it.
    // Inserting text past the marker must NOT shift the start.
    let mut d = doc_with_one_footnote(5, "annot");
    let id = d.last_footnote_id().unwrap();
    // Insert at the END of the body — well past the marker.
    d.insert(d.text().chars().count(), "tail");
    let r = d.find_footnote_range(&id).expect("mark survives");
    assert_eq!(r.start, 5);
    assert_eq!(r.end - r.start, 1);
}

// ─────────────────────────────────────────────────────────────────
// footnotes_in_body_order — position-order snapshot
// ─────────────────────────────────────────────────────────────────

#[test]
fn footnotes_in_body_order_returns_empty_for_doc_without_footnotes() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    assert!(d.footnotes_in_body_order().is_empty());
}

#[test]
fn footnotes_in_body_order_returns_one_entry_per_footnote() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 1,
        body: BlockTree {
            blocks: vec![paragraph("a")],
        },
    })
    .unwrap();
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 3, // after the first marker, body is now "a<m1>bc"
        body: BlockTree {
            blocks: vec![paragraph("b")],
        },
    })
    .unwrap();
    let entries: Vec<Footnote> = d.footnotes_in_body_order();
    assert_eq!(entries.len(), 2);
}

#[test]
fn footnotes_in_body_order_sorts_by_current_marker_position() {
    // Insert footnote A at the END of the body, then footnote B at
    // the START. Position order must put B FIRST (it has the
    // lower-position marker), regardless of insertion (id) order.
    let mut d = Doc::new();
    d.insert(0, "hello world");
    // A → marker at position 11 (end)
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 11,
        body: BlockTree {
            blocks: vec![paragraph("body-A")],
        },
    })
    .unwrap();
    let id_a = d.last_footnote_id().unwrap();
    // B → marker at position 0 (start) — body length is now 12
    d.apply_edit_op(EditOp::InsertFootnote {
        at: 0,
        body: BlockTree {
            blocks: vec![paragraph("body-B")],
        },
    })
    .unwrap();
    let id_b = d.last_footnote_id().unwrap();
    let order: Vec<String> = d
        .footnotes_in_body_order()
        .iter()
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(order, vec![id_b, id_a]);
}

// ─────────────────────────────────────────────────────────────────
// Property: snapshot round-trip preserves footnote + mark
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// After InsertFootnote, snapshot/import must preserve the
    /// footnote's metadata AND the mark on the marker codepoint.
    /// Catches mutations in the storage path that would, eg., drop
    /// the mark on the wire (losing anchor stability after reload).
    #[test]
    fn prop_snapshot_round_trip_preserves_footnote_mark(
        body_text in "\\PC{0,16}",
        at in 0usize..5,
    ) {
        let mut d = Doc::new();
        d.insert(0, "hello");
        d.apply_edit_op(EditOp::InsertFootnote {
            at,
            body: BlockTree { blocks: vec![paragraph(&body_text)] },
        }).unwrap();
        let id = d.last_footnote_id().unwrap();
        let original = d.footnote(&id).expect("original present");
        let snap = d.snapshot();
        let restored = Doc::from_snapshot(&snap);
        let restored_f = restored.footnote(&id).expect("restored present");
        prop_assert_eq!(original, restored_f);
        prop_assert_eq!(restored.find_footnote_range(&id), Some(at..at + 1));
    }
}
