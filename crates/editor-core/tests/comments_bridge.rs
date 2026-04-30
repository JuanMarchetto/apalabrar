//! Phase 4.6 RED — bridge JSON for ReplyToComment / SetCommentStatus
//! / comments_json.
//!
//! These tests live in their OWN integration-test binary so the
//! `todo!()` panics that fire in step-1 spec stubs don't poison the
//! shared-state mutex used by `tests/bridge.rs`. Each integration
//! test file gets its own process; the registry is per-process.

use apalabrar_doc_model::{CommentStatus, EditOp};
use apalabrar_editor_core::bridge::{
    apply_edit_op, apply_edit_op_json, comments_json, create_doc, with_doc,
};
use apalabrar_editor_core::{Error, close_doc};

#[test]
fn apply_edit_op_json_dispatches_reply_to_comment() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertComment {
            from: 0,
            to: 0,
            body: "head".into(),
            thread_id: Some("t-1".into()),
            author: "alice".into(),
            created_at: 1,
        },
    )
    .unwrap();
    let op_json = r#"{
        "kind": "ReplyToComment",
        "thread_id": "t-1",
        "body": "agreed",
        "author": "bob",
        "created_at": 2
    }"#;
    apply_edit_op_json(id, op_json).unwrap();
    let reply_count = with_doc(id, |d| {
        d.comment("t-1").map(|c| c.replies.len()).unwrap_or(0)
    })
    .unwrap();
    assert_eq!(reply_count, 1);
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_json_dispatches_set_comment_status() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertComment {
            from: 0,
            to: 0,
            body: "head".into(),
            thread_id: Some("t-1".into()),
            author: "alice".into(),
            created_at: 1,
        },
    )
    .unwrap();
    let op_json = r#"{"kind":"SetCommentStatus","thread_id":"t-1","status":"resolved"}"#;
    apply_edit_op_json(id, op_json).unwrap();
    let status = with_doc(id, |d| d.comment("t-1").map(|c| c.status)).unwrap();
    assert_eq!(status, Some(CommentStatus::Resolved));
    close_doc(id).unwrap();
}

#[test]
fn comments_json_returns_sorted_thread_array_with_full_shape() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertComment {
            from: 0,
            to: 0,
            body: "B".into(),
            thread_id: Some("t-b".into()),
            author: "alice".into(),
            created_at: 1,
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::InsertComment {
            from: 0,
            to: 0,
            body: "A".into(),
            thread_id: Some("t-a".into()),
            author: "bob".into(),
            created_at: 2,
        },
    )
    .unwrap();
    let json = comments_json(id).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let arr = parsed
        .as_array()
        .expect("comments_json must be a JSON array");
    assert_eq!(arr.len(), 2, "expected 2 threads, got {arr:?}");
    // Sorted by thread_id, ascending.
    assert_eq!(arr[0]["thread_id"], "t-a");
    assert_eq!(arr[1]["thread_id"], "t-b");
    // Snapshot shape: each thread carries the new fields.
    assert_eq!(arr[0]["author"], "bob");
    assert_eq!(arr[0]["created_at"], 2);
    assert_eq!(arr[0]["status"], "open");
    assert!(arr[0]["replies"].is_array());
    close_doc(id).unwrap();
}

#[test]
fn comments_json_on_unknown_doc_returns_unknown_doc_error() {
    let id = create_doc();
    close_doc(id).unwrap();
    let err = comments_json(id).unwrap_err();
    assert!(matches!(err, Error::UnknownDoc(_)));
}
