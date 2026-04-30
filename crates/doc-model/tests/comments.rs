//! Phase 4.6 RED — comment threading tests.
//!
//! Covers: author/timestamp persistence on InsertComment, ReplyToComment
//! happy path + errors, SetCommentStatus open/resolved/idempotent/error,
//! comments() accessor, properties (snapshot determinism, reply order
//! preserved under merge).

use apalabrar_doc_model::{Comment, CommentReply, CommentStatus, Doc, EditOp, Error};
use proptest::prelude::*;

// Sensible default fixture: a doc with one open thread "t-base".
fn doc_with_thread(thread_id: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.apply_edit_op(EditOp::InsertComment {
        from: 0,
        to: 5,
        body: "first".into(),
        thread_id: Some(thread_id.into()),
        author: "alice".into(),
        created_at: 1_000,
    })
    .unwrap();
    d
}

// ─────────────────────────────────────────────────────────────────
// InsertComment with author / created_at persistence
// ─────────────────────────────────────────────────────────────────

#[test]
fn insert_comment_persists_author_and_created_at_on_snapshot() {
    let d = doc_with_thread("t-1");
    let c = d.comment("t-1").expect("comment present");
    assert_eq!(c.author, "alice");
    assert_eq!(c.created_at, 1_000);
}

#[test]
fn insert_comment_default_status_is_open() {
    let d = doc_with_thread("t-1");
    let c = d.comment("t-1").expect("comment present");
    assert_eq!(c.status, CommentStatus::Open);
}

#[test]
fn insert_comment_default_replies_is_empty() {
    let d = doc_with_thread("t-1");
    let c = d.comment("t-1").expect("comment present");
    assert_eq!(c.replies, Vec::<CommentReply>::new());
}

// ─────────────────────────────────────────────────────────────────
// ReplyToComment
// ─────────────────────────────────────────────────────────────────

#[test]
fn reply_appends_a_reply_to_the_thread() {
    let mut d = doc_with_thread("t-1");
    d.apply_edit_op(EditOp::ReplyToComment {
        thread_id: "t-1".into(),
        body: "agreed".into(),
        author: "bob".into(),
        created_at: 2_000,
    })
    .unwrap();
    let c = d.comment("t-1").expect("comment present");
    assert_eq!(c.replies.len(), 1);
    assert_eq!(c.replies[0].body, "agreed");
    assert_eq!(c.replies[0].author, "bob");
    assert_eq!(c.replies[0].created_at, 2_000);
}

#[test]
fn reply_to_unknown_thread_returns_comment_not_found() {
    let mut d = Doc::new();
    let err = d
        .apply_edit_op(EditOp::ReplyToComment {
            thread_id: "t-bogus".into(),
            body: "x".into(),
            author: "carol".into(),
            created_at: 3_000,
        })
        .unwrap_err();
    assert_eq!(err, Error::CommentNotFound("t-bogus".into()));
}

#[test]
fn reply_sets_last_reply_id_to_a_minted_string() {
    let mut d = doc_with_thread("t-1");
    d.apply_edit_op(EditOp::ReplyToComment {
        thread_id: "t-1".into(),
        body: "x".into(),
        author: "carol".into(),
        created_at: 4_000,
    })
    .unwrap();
    let id = d
        .last_reply_id()
        .expect("last_reply_id should be Some after reply");
    assert!(!id.is_empty(), "minted reply id must not be empty");
}

#[test]
fn reply_ids_are_unique_across_consecutive_replies() {
    let mut d = doc_with_thread("t-1");
    let mut ids = Vec::new();
    for n in 0..3 {
        d.apply_edit_op(EditOp::ReplyToComment {
            thread_id: "t-1".into(),
            body: format!("reply {n}"),
            author: "x".into(),
            created_at: 1_000 + n,
        })
        .unwrap();
        ids.push(d.last_reply_id().unwrap());
    }
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "reply ids must be unique: {ids:?}");
}

#[test]
fn replies_preserve_insertion_order() {
    let mut d = doc_with_thread("t-1");
    for n in 0..4 {
        d.apply_edit_op(EditOp::ReplyToComment {
            thread_id: "t-1".into(),
            body: format!("r{n}"),
            author: "x".into(),
            created_at: 1_000 + n,
        })
        .unwrap();
    }
    let c = d.comment("t-1").unwrap();
    let bodies: Vec<&str> = c.replies.iter().map(|r| r.body.as_str()).collect();
    assert_eq!(bodies, vec!["r0", "r1", "r2", "r3"]);
}

// ─────────────────────────────────────────────────────────────────
// SetCommentStatus
// ─────────────────────────────────────────────────────────────────

#[test]
fn set_status_resolved_flips_status() {
    let mut d = doc_with_thread("t-1");
    d.apply_edit_op(EditOp::SetCommentStatus {
        thread_id: "t-1".into(),
        status: CommentStatus::Resolved,
    })
    .unwrap();
    assert_eq!(d.comment("t-1").unwrap().status, CommentStatus::Resolved);
}

#[test]
fn set_status_open_after_resolved_unresolves() {
    let mut d = doc_with_thread("t-1");
    d.apply_edit_op(EditOp::SetCommentStatus {
        thread_id: "t-1".into(),
        status: CommentStatus::Resolved,
    })
    .unwrap();
    d.apply_edit_op(EditOp::SetCommentStatus {
        thread_id: "t-1".into(),
        status: CommentStatus::Open,
    })
    .unwrap();
    assert_eq!(d.comment("t-1").unwrap().status, CommentStatus::Open);
}

#[test]
fn set_status_idempotent_resolve_resolved_thread_is_noop() {
    let mut d = doc_with_thread("t-1");
    d.apply_edit_op(EditOp::SetCommentStatus {
        thread_id: "t-1".into(),
        status: CommentStatus::Resolved,
    })
    .unwrap();
    // Apply again — must not error and must keep status.
    d.apply_edit_op(EditOp::SetCommentStatus {
        thread_id: "t-1".into(),
        status: CommentStatus::Resolved,
    })
    .unwrap();
    assert_eq!(d.comment("t-1").unwrap().status, CommentStatus::Resolved);
}

#[test]
fn set_status_unknown_thread_returns_comment_not_found() {
    let mut d = Doc::new();
    let err = d
        .apply_edit_op(EditOp::SetCommentStatus {
            thread_id: "t-bogus".into(),
            status: CommentStatus::Resolved,
        })
        .unwrap_err();
    assert_eq!(err, Error::CommentNotFound("t-bogus".into()));
}

// ─────────────────────────────────────────────────────────────────
// comments() accessor
// ─────────────────────────────────────────────────────────────────

#[test]
fn comments_returns_empty_for_a_fresh_doc() {
    let d = Doc::new();
    assert_eq!(d.comments(), Vec::<Comment>::new());
}

#[test]
fn comments_returns_threads_sorted_by_thread_id() {
    let mut d = Doc::new();
    d.insert(0, "abc");
    for tid in ["t-z", "t-a", "t-m"] {
        d.apply_edit_op(EditOp::InsertComment {
            from: 0,
            to: 1,
            body: tid.into(),
            thread_id: Some(tid.into()),
            author: "alice".into(),
            created_at: 0,
        })
        .unwrap();
    }
    let ids: Vec<String> = d.comments().into_iter().map(|c| c.thread_id).collect();
    assert_eq!(ids, vec!["t-a", "t-m", "t-z"]);
}

// ─────────────────────────────────────────────────────────────────
// Snapshot determinism property
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Snapshot/import round-trip preserves the `replies` ordering,
    /// status, author, and created_at on every thread. Catches a
    /// mutation in the storage path that would, eg., collect replies
    /// into a HashSet (losing order) or drop status when serialising.
    #[test]
    fn prop_snapshot_round_trip_preserves_thread_state(
        n_replies in 0usize..6,
        resolved in any::<bool>(),
    ) {
        let mut d = Doc::new();
        d.insert(0, "x");
        d.apply_edit_op(EditOp::InsertComment {
            from: 0, to: 1,
            body: "head".into(),
            thread_id: Some("t".into()),
            author: "alice".into(),
            created_at: 100,
        }).unwrap();
        for n in 0..n_replies {
            d.apply_edit_op(EditOp::ReplyToComment {
                thread_id: "t".into(),
                body: format!("r{n}"),
                author: "bob".into(),
                created_at: 200 + n as i64,
            }).unwrap();
        }
        if resolved {
            d.apply_edit_op(EditOp::SetCommentStatus {
                thread_id: "t".into(),
                status: CommentStatus::Resolved,
            }).unwrap();
        }
        let snap = d.snapshot();
        let restored = Doc::from_snapshot(&snap);
        let original = d.comment("t").expect("original");
        let restored = restored.comment("t").expect("restored");
        // Full struct equality also checks replies as Vec (preserves order).
        prop_assert_eq!(&original, &restored);
        // Sharper: explicitly assert the `bodies` vector matches in
        // order. If a buggy storage path used a HashMap (losing
        // order), the Vec equality above would still fail, but this
        // line names the specific invariant in the failure message.
        let bodies: Vec<&str> = original.replies.iter().map(|r| r.body.as_str()).collect();
        let restored_bodies: Vec<&str> =
            restored.replies.iter().map(|r| r.body.as_str()).collect();
        prop_assert_eq!(bodies, restored_bodies);
    }
}
