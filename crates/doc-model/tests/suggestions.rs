//! Phase 4.7 RED — track changes (suggestions) tests.
//!
//! Locked semantics:
//! - suggestions are anchored by a Loro mark with key "suggested"
//!   and value = suggestion_id; the mark is the authoritative range
//! - Suggestion snapshot carries author + created_at (Phase 4.7)
//! - `EditOp::RejectSuggestion` removes the mark, sets state to
//!   Rejected, leaves text untouched, is one-way
//! - Accept after edit-inside uses the *current* mark range, not
//!   the stored from/to
//! - Idempotent: accept/reject on already-accepted/rejected → Ok no-op

use apalabrar_doc_model::{Doc, EditOp, Error, Suggestion, SuggestionState};
use proptest::prelude::*;

fn doc_with_suggestion(replacement: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 0,
        to: 5,
        replacement: replacement.into(),
        author: "alice".into(),
        created_at: 1_000,
    })
    .unwrap();
    d
}

// ─────────────────────────────────────────────────────────────────
// Suggest with author / created_at persistence
// ─────────────────────────────────────────────────────────────────

#[test]
fn suggest_persists_author_on_snapshot() {
    let d = doc_with_suggestion("HOLA");
    let id = d.last_suggestion_id().unwrap();
    let s = d.suggestion(&id).expect("suggestion present");
    assert_eq!(s.author, "alice");
}

#[test]
fn suggest_persists_created_at_on_snapshot() {
    let d = doc_with_suggestion("HOLA");
    let id = d.last_suggestion_id().unwrap();
    let s = d.suggestion(&id).expect("suggestion present");
    assert_eq!(s.created_at, 1_000);
}

// ─────────────────────────────────────────────────────────────────
// Marks anchoring — find_suggestion_range / suggestion_at
// ─────────────────────────────────────────────────────────────────

#[test]
fn find_suggestion_range_returns_the_marked_range_after_suggest() {
    let d = doc_with_suggestion("HOLA");
    let id = d.last_suggestion_id().unwrap();
    let r = d.find_suggestion_range(&id).expect("mark must exist");
    // "hello world" — suggestion was on [0..5) ("hello").
    assert_eq!(r, 0..5);
}

#[test]
fn find_suggestion_range_returns_none_for_unknown_id() {
    let d = Doc::new();
    assert_eq!(d.find_suggestion_range("s-bogus"), None);
}

#[test]
fn suggestion_at_returns_id_for_position_inside_marked_range() {
    let d = doc_with_suggestion("X");
    let id = d.last_suggestion_id().unwrap();
    // pos=2 is inside [0..5)
    assert_eq!(d.suggestion_at(2), Some(id));
}

#[test]
fn suggestion_at_returns_none_for_position_outside_marked_range() {
    let d = doc_with_suggestion("X");
    // pos=8 is in "world", outside the marked [0..5) range.
    assert_eq!(d.suggestion_at(8), None);
}

#[test]
fn suggestion_at_returns_none_for_position_exactly_at_end_of_marked_range() {
    // Mutation guard: the boundary check must be `pos < cursor + chars`,
    // not `<=`. Mark covers [0..5) — pos=5 is the FIRST char of the next
    // segment (' '), NOT part of the mark.
    let d = doc_with_suggestion("X");
    assert_eq!(d.suggestion_at(5), None);
}

#[test]
fn suggestion_at_finds_mark_on_a_non_first_segment() {
    // Mutation guard: the cursor advance `cursor += chars` must be
    // additive, not multiplicative. Place the mark on the SECOND
    // segment so the loop has to advance past segment 0 to find it.
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::Suggest {
        from: 6,
        to: 11,
        replacement: "X".into(),
        author: "alice".into(),
        created_at: 1,
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    // pos=8 ('r' in "world") — after to_delta yields "hello "
    // (unmarked, 6 chars) + "world" (marked, 5 chars), the loop must
    // bump cursor from 0 to 6 to find the mark covering pos=8.
    assert_eq!(d.suggestion_at(8), Some(id));
}

#[test]
fn suggest_with_zero_width_range_does_not_apply_a_mark() {
    // Mutation guard: handle_suggest must skip body.mark() when
    // `start == end` — applying a zero-width mark would either be a
    // wasteful CRDT op or create a phantom mark with no covered text.
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.apply_edit_op(EditOp::Suggest {
        from: 2,
        to: 2,
        replacement: "X".into(),
        author: "alice".into(),
        created_at: 1,
    })
    .unwrap();
    let id = d.last_suggestion_id().unwrap();
    // No codepoints carry the mark; find_suggestion_range returns None.
    assert_eq!(d.find_suggestion_range(&id), None);
}

#[test]
fn find_suggestion_range_tracks_text_inserted_inside() {
    // Critical anchor-stability test: insert text inside the marked
    // range; the mark must expand to include it (Loro mark semantics).
    // Stored from/to remain the original (0, 5); but
    // find_suggestion_range should reflect the current span.
    let mut d = doc_with_suggestion("HOLA");
    let id = d.last_suggestion_id().unwrap();
    // Insert "X" at codepoint 2 — original "hello" becomes "heXllo".
    d.insert(2, "X");
    let r = d.find_suggestion_range(&id).expect("mark survives insert");
    // The marked range now covers 6 codepoints (h e X l l o).
    assert_eq!(r.end - r.start, 6);
}

// ─────────────────────────────────────────────────────────────────
// AcceptSuggestion uses the CURRENT mark range, not stored from/to
// ─────────────────────────────────────────────────────────────────

#[test]
fn accept_after_inserting_inside_uses_current_marked_range() {
    let mut d = doc_with_suggestion("WORLD");
    let id = d.last_suggestion_id().unwrap();
    // Edit inside the marked range: "hello" -> "heXllo".
    d.insert(2, "X");
    assert_eq!(d.text(), "heXllo world");
    // Accept: must replace the CURRENT marked range ("heXllo") with
    // "WORLD", NOT the original (0..5) which would only cover "heXll".
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.text(), "WORLD world");
    assert_eq!(d.suggestion(&id).unwrap().state, SuggestionState::Accepted);
}

#[test]
fn accept_clears_the_suggested_mark() {
    let mut d = doc_with_suggestion("X");
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.find_suggestion_range(&id), None);
}

// ─────────────────────────────────────────────────────────────────
// RejectSuggestion
// ─────────────────────────────────────────────────────────────────

#[test]
fn reject_does_not_mutate_body_text() {
    let mut d = doc_with_suggestion("WORLD");
    let id = d.last_suggestion_id().unwrap();
    let before = d.text();
    d.apply_edit_op(EditOp::RejectSuggestion { suggestion_id: id })
        .unwrap();
    assert_eq!(d.text(), before);
}

#[test]
fn reject_sets_state_to_rejected() {
    let mut d = doc_with_suggestion("X");
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::RejectSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.suggestion(&id).unwrap().state, SuggestionState::Rejected);
}

#[test]
fn reject_clears_the_suggested_mark() {
    let mut d = doc_with_suggestion("X");
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::RejectSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.find_suggestion_range(&id), None);
}

#[test]
fn reject_unknown_suggestion_returns_suggestion_not_found() {
    let mut d = Doc::new();
    let err = d
        .apply_edit_op(EditOp::RejectSuggestion {
            suggestion_id: "s-bogus".into(),
        })
        .unwrap_err();
    assert_eq!(err, Error::SuggestionNotFound("s-bogus".into()));
}

#[test]
fn reject_already_accepted_is_idempotent_noop() {
    let mut d = doc_with_suggestion("X");
    let id = d.last_suggestion_id().unwrap();
    d.apply_edit_op(EditOp::AcceptSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    // Reject after accept should NOT flip state back to Rejected — it's
    // a no-op for already-resolved suggestions.
    d.apply_edit_op(EditOp::RejectSuggestion {
        suggestion_id: id.clone(),
    })
    .unwrap();
    assert_eq!(d.suggestion(&id).unwrap().state, SuggestionState::Accepted);
}

// ─────────────────────────────────────────────────────────────────
// Property: snapshot round-trip preserves suggestion + mark
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// After Suggest, snapshot/import must preserve the suggestion's
    /// metadata AND the mark on the original range. Catches mutations
    /// in the storage path that would, eg., serialise marks as
    /// boolean-only (losing the id value).
    #[test]
    fn prop_snapshot_round_trip_preserves_suggestion_state(
        replacement in "\\PC{0,12}",
        author in "\\PC{1,8}",
        created_at in 0i64..1_000_000,
    ) {
        let mut d = Doc::new();
        d.insert(0, "hello world");
        d.apply_edit_op(EditOp::Suggest {
            from: 0, to: 5,
            replacement: replacement.clone(),
            author: author.clone(),
            created_at,
        }).unwrap();
        let id = d.last_suggestion_id().unwrap();
        let original = d.suggestion(&id).expect("original present");
        let snap = d.snapshot();
        let restored = Doc::from_snapshot(&snap);
        let restored_s: Suggestion = restored.suggestion(&id).expect("restored present");
        prop_assert_eq!(original, restored_s);
        prop_assert_eq!(restored.find_suggestion_range(&id), Some(0..5));
    }
}
