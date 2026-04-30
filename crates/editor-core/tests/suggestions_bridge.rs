//! Phase 4.7 RED — bridge JSON for RejectSuggestion + suggestions_json.
//!
//! Separate integration-test binary so the `todo!()` panic in
//! `suggestions_json` (and `handle_reject_suggestion`) doesn't poison
//! the registry mutex shared across `tests/bridge.rs`.

use apalabrar_doc_model::{EditOp, SuggestionState};
use apalabrar_editor_core::bridge::{
    apply_edit_op, apply_edit_op_json, create_doc, suggestions_json, with_doc,
};
use apalabrar_editor_core::{Error, close_doc};

#[test]
fn apply_edit_op_json_dispatches_reject_suggestion() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "hello world".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::Suggest {
            from: 0,
            to: 5,
            replacement: "X".into(),
            author: "alice".into(),
            created_at: 1,
        },
    )
    .unwrap();
    let suggestion_id = with_doc(id, |d| d.last_suggestion_id().unwrap()).unwrap();
    let op_json = format!(r#"{{"kind":"RejectSuggestion","suggestion_id":"{suggestion_id}"}}"#,);
    apply_edit_op_json(id, &op_json).unwrap();
    let state = with_doc(id, |d| d.suggestion(&suggestion_id).map(|s| s.state)).unwrap();
    assert_eq!(state, Some(SuggestionState::Rejected));
    close_doc(id).unwrap();
}

#[test]
fn suggestions_json_returns_sorted_suggestion_array_with_full_shape() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "hello world".into(),
            marks: vec![],
        },
    )
    .unwrap();
    // Two suggestions; ids are auto-minted "s-..." in insertion order.
    apply_edit_op(
        id,
        EditOp::Suggest {
            from: 0,
            to: 5,
            replacement: "B".into(),
            author: "bob".into(),
            created_at: 100,
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::Suggest {
            from: 6,
            to: 11,
            replacement: "A".into(),
            author: "alice".into(),
            created_at: 200,
        },
    )
    .unwrap();
    let json = suggestions_json(id).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let arr = parsed
        .as_array()
        .expect("suggestions_json must be a JSON array");
    assert_eq!(arr.len(), 2);
    // Each carries the new fields with the values we wrote in.
    // (We don't know which id is sorted first, so assert as a set.)
    let authors: std::collections::HashSet<&str> = arr
        .iter()
        .map(|v| v["author"].as_str().expect("author string"))
        .collect();
    assert_eq!(
        authors,
        ["alice", "bob"]
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
    );
    let created_ats: std::collections::HashSet<i64> = arr
        .iter()
        .map(|v| v["created_at"].as_i64().expect("created_at i64"))
        .collect();
    assert_eq!(created_ats, [100i64, 200].iter().copied().collect());
    // State is a lowercase serde-rename string.
    assert_eq!(arr[0]["state"], "pending");
    close_doc(id).unwrap();
}

#[test]
fn suggestions_json_on_unknown_doc_returns_unknown_doc_error() {
    let id = create_doc();
    close_doc(id).unwrap();
    let err = suggestions_json(id).unwrap_err();
    assert!(matches!(err, Error::UnknownDoc(_)));
}

#[test]
fn suggestions_json_returns_empty_array_for_doc_without_suggestions() {
    let id = create_doc();
    let json = suggestions_json(id).unwrap();
    assert_eq!(json, "[]");
    close_doc(id).unwrap();
}
