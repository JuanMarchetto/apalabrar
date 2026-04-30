//! RED-phase tests for the Phase 2.3 bridge module.
//!
//! The `bridge` module exposes the doc-model EditOp surface (Phase 2.2's
//! 11 variants) through editor-core's registry, so the JS side can apply
//! edits via `apply_edit_op_json` without leaving the doc-model abstraction.
//!
//! Tests here are HOST-side (regular Rust tests). The wasm-bindgen
//! exports are smoke-tested separately in `tests/wasm.rs` once a browser
//! runtime is available; their underlying logic is exercised through the
//! same public Rust API tested here.

use apalabrar_doc_model::{Block, BlockKind, BlockTree, EditOp, Mark, SuggestionState};
use apalabrar_editor_core::bridge::{
    apply_edit_op, apply_edit_op_json, block_at_json, block_count, create_doc,
    restore_from_snapshot, snapshot,
};
use apalabrar_editor_core::{Error, close_doc, doc_text};

// ─────────────────────────────────────────────────────────────────
// Lifecycle
// ─────────────────────────────────────────────────────────────────

#[test]
fn create_doc_returns_a_fresh_doc_id() {
    let id = create_doc();
    assert_eq!(doc_text(id).unwrap(), "");
    close_doc(id).unwrap();
}

#[test]
fn create_doc_returns_distinct_ids_across_calls() {
    let a = create_doc();
    let b = create_doc();
    assert_ne!(a, b);
    close_doc(a).unwrap();
    close_doc(b).unwrap();
}

#[test]
fn snapshot_returns_bytes_for_empty_doc() {
    let id = create_doc();
    let bytes = snapshot(id).unwrap();
    assert!(!bytes.is_empty(), "snapshot must include CRDT metadata");
    close_doc(id).unwrap();
}

#[test]
fn restore_from_snapshot_round_trips_text_and_blocks() {
    let original = create_doc();
    apply_edit_op(
        original,
        EditOp::InsertText {
            at: 0,
            text: "alpha\nbeta".into(),
            marks: vec![],
        },
    )
    .unwrap();
    let snap = snapshot(original).unwrap();
    let restored = restore_from_snapshot(&snap).unwrap();
    assert_eq!(doc_text(restored).unwrap(), "alpha\nbeta");
    assert_eq!(block_count(restored).unwrap(), 2);
    close_doc(original).unwrap();
    close_doc(restored).unwrap();
}

#[test]
fn restore_from_snapshot_with_garbage_bytes_returns_error() {
    let result = restore_from_snapshot(&[1, 2, 3, 4, 5]);
    assert!(result.is_err(), "garbage bytes must not produce a DocId");
}

#[test]
fn snapshot_unknown_doc_returns_error() {
    use apalabrar_editor_core::DocId;
    // Build a clearly-invalid handle by closing one.
    let id = create_doc();
    close_doc(id).unwrap();
    assert!(matches!(snapshot(id), Err(Error::UnknownDoc(_))));
    let _: DocId = id; // keep type info
}

// ─────────────────────────────────────────────────────────────────
// EditOp dispatch — one happy path per variant
// ─────────────────────────────────────────────────────────────────

#[test]
fn apply_edit_op_insert_text_into_empty_doc() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "hello".into(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_insert_text_with_marks() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "rich".into(),
            marks: vec![Mark::Bold, Mark::Italic],
        },
    )
    .unwrap();
    // Text projection has the inserted span.
    assert_eq!(doc_text(id).unwrap(), "rich");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_delete_range_removes_chars() {
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
    apply_edit_op(id, EditOp::DeleteRange { from: 5, to: 11 }).unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_format_range_succeeds() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "abc".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::FormatRange {
            from: 0,
            to: 3,
            mark: Mark::Bold,
        },
    )
    .unwrap();
    // Text unchanged; the mark is in the rich-text projection.
    assert_eq!(doc_text(id).unwrap(), "abc");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_insert_block_at_start() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "body".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::InsertBlock {
            at: 0,
            block: Block {
                kind: BlockKind::Heading { level: 1 },
                text: "title".into(),
            },
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "title\nbody");
    assert_eq!(block_count(id).unwrap(), 2);
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_split_block_at_position() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "abcdef".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(id, EditOp::SplitBlock { at: 3 }).unwrap();
    assert_eq!(doc_text(id).unwrap(), "abc\ndef");
    assert_eq!(block_count(id).unwrap(), 2);
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_merge_blocks_combines_text() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "hello\nworld".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::MergeBlocks {
            first: 0,
            second: 6,
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "helloworld");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_insert_comment_with_explicit_thread_id() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "abc".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::InsertComment {
            from: 0,
            to: 1,
            body: "review".into(),
            thread_id: Some("t-1".into()),
            author: "tester".into(),
            created_at: 0,
        },
    )
    .unwrap();
    // Body unchanged by the comment op.
    assert_eq!(doc_text(id).unwrap(), "abc");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_suggest_then_accept_applies_replacement() {
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
            replacement: "HOLA".into(),
        },
    )
    .unwrap();
    // Suggest does NOT mutate the body yet.
    assert_eq!(doc_text(id).unwrap(), "hello world");
    // Read the assigned id from the underlying doc-model.
    let suggestion_id = apalabrar_editor_core::bridge::with_doc(id, |d| {
        d.last_suggestion_id().expect("id assigned")
    })
    .unwrap();
    apply_edit_op(
        id,
        EditOp::AcceptSuggestion {
            suggestion_id: suggestion_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "HOLA world");
    let state = apalabrar_editor_core::bridge::with_doc(id, |d| {
        d.suggestion(&suggestion_id).map(|s| s.state)
    })
    .unwrap();
    assert_eq!(state, Some(SuggestionState::Accepted));
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_insert_citation_inserts_marker() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "ref".into(),
            marks: vec![],
        },
    )
    .unwrap();
    apply_edit_op(
        id,
        EditOp::InsertCitation {
            at: 3,
            key: "Smith2020".into(),
        },
    )
    .unwrap();
    let body = doc_text(id).unwrap();
    let chars: Vec<char> = body.chars().collect();
    assert_eq!(chars.len(), 4);
    assert_eq!(chars[3], '\u{E000}');
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_insert_footnote_inserts_marker_and_records_body() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "main".into(),
            marks: vec![],
        },
    )
    .unwrap();
    let body = BlockTree {
        blocks: vec![Block {
            kind: BlockKind::Paragraph,
            text: "footnote text".into(),
        }],
    };
    apply_edit_op(id, EditOp::InsertFootnote { at: 4, body }).unwrap();
    let chars: Vec<char> = doc_text(id).unwrap().chars().collect();
    assert_eq!(chars.last().copied(), Some('\u{E001}'));
    close_doc(id).unwrap();
}

// ─────────────────────────────────────────────────────────────────
// JSON dispatcher
// ─────────────────────────────────────────────────────────────────

#[test]
fn apply_edit_op_json_routes_insert_text() {
    let id = create_doc();
    let json = r#"{"kind":"InsertText","at":0,"text":"json","marks":[]}"#;
    apply_edit_op_json(id, json).unwrap();
    assert_eq!(doc_text(id).unwrap(), "json");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_json_routes_format_range_with_mark() {
    let id = create_doc();
    apply_edit_op_json(
        id,
        r#"{"kind":"InsertText","at":0,"text":"abc","marks":[]}"#,
    )
    .unwrap();
    apply_edit_op_json(
        id,
        r#"{"kind":"FormatRange","from":0,"to":3,"mark":"Italic"}"#,
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "abc");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_json_routes_insert_block_with_heading_kind() {
    let id = create_doc();
    apply_edit_op_json(
        id,
        r#"{"kind":"InsertBlock","at":0,"block":{"kind":{"type":"Heading","level":2},"text":"H"}}"#,
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "H\n");
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_json_invalid_json_returns_parse_error() {
    let id = create_doc();
    let result = apply_edit_op_json(id, "not valid json{");
    assert!(matches!(result, Err(Error::JsonParseFailed { .. })));
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_json_unknown_variant_returns_parse_error() {
    let id = create_doc();
    let result = apply_edit_op_json(id, r#"{"kind":"NotARealVariant","at":0}"#);
    assert!(matches!(result, Err(Error::JsonParseFailed { .. })));
    close_doc(id).unwrap();
}

#[test]
fn apply_edit_op_json_round_trip_preserves_op() {
    // Serialize an EditOp via serde_json, hand it to apply_edit_op_json,
    // and confirm the doc state matches the equivalent native call.
    let op = EditOp::InsertText {
        at: 0,
        text: "rt".into(),
        marks: vec![Mark::Bold],
    };
    let op_json = serde_json::to_string(&op).expect("EditOp serializes");
    let id_json = create_doc();
    apply_edit_op_json(id_json, &op_json).unwrap();
    let id_native = create_doc();
    apply_edit_op(id_native, op).unwrap();
    assert_eq!(doc_text(id_json).unwrap(), doc_text(id_native).unwrap(),);
    close_doc(id_json).unwrap();
    close_doc(id_native).unwrap();
}

// ─────────────────────────────────────────────────────────────────
// Queries
// ─────────────────────────────────────────────────────────────────

#[test]
fn block_count_empty_doc_is_one() {
    let id = create_doc();
    assert_eq!(block_count(id).unwrap(), 1);
    close_doc(id).unwrap();
}

#[test]
fn block_count_after_inserts_with_newlines() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "a\nb\nc".into(),
            marks: vec![],
        },
    )
    .unwrap();
    assert_eq!(block_count(id).unwrap(), 3);
    close_doc(id).unwrap();
}

#[test]
fn block_at_json_returns_serialized_block() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertBlock {
            at: 0,
            block: Block {
                kind: BlockKind::Heading { level: 1 },
                text: "title".into(),
            },
        },
    )
    .unwrap();
    let json = block_at_json(id, 0).unwrap().expect("block 0 exists");
    // Must be parseable back into a Block.
    let block: Block = serde_json::from_str(&json).expect("parses");
    assert_eq!(block.text, "title");
    assert_eq!(block.kind, BlockKind::Heading { level: 1 });
    close_doc(id).unwrap();
}

#[test]
fn block_at_json_out_of_range_returns_none() {
    let id = create_doc();
    let result = block_at_json(id, 999).unwrap();
    assert!(result.is_none());
    close_doc(id).unwrap();
}

// ─────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────

#[test]
fn apply_edit_op_unknown_doc_returns_unknown_doc_error() {
    let id = create_doc();
    close_doc(id).unwrap();
    let result = apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "x".into(),
            marks: vec![],
        },
    );
    assert!(matches!(result, Err(Error::UnknownDoc(_))));
}

#[test]
fn apply_edit_op_accept_suggestion_unknown_id_returns_edit_op_failed() {
    let id = create_doc();
    let result = apply_edit_op(
        id,
        EditOp::AcceptSuggestion {
            suggestion_id: "no-such-id".into(),
        },
    );
    let Err(Error::EditOpFailed { kind, reason }) = result else {
        panic!("expected EditOpFailed, got {result:?}");
    };
    // Pin the variant-name string and the underlying reason so the
    // bridge layer's `edit_op_kind` and the doc-model error message
    // both stay in sync — catches kind-rename mutations and any
    // change to the doc-model error format.
    assert_eq!(kind, "AcceptSuggestion");
    assert!(
        reason.contains("no-such-id"),
        "reason should include the bad id: {reason}"
    );
    close_doc(id).unwrap();
}

#[test]
fn block_count_unknown_doc_returns_unknown_doc_error() {
    let id = create_doc();
    close_doc(id).unwrap();
    assert!(matches!(block_count(id), Err(Error::UnknownDoc(_))));
}

#[test]
fn block_at_json_unknown_doc_returns_unknown_doc_error() {
    let id = create_doc();
    close_doc(id).unwrap();
    assert!(matches!(block_at_json(id, 0), Err(Error::UnknownDoc(_))));
}

// ─────────────────────────────────────────────────────────────────
// Properties
// ─────────────────────────────────────────────────────────────────

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        ..ProptestConfig::default()
    })]

    /// Round-trip property: serialize an InsertText op via serde_json,
    /// dispatch through `apply_edit_op_json`, and verify the resulting
    /// doc text matches a native dispatch of the same op.
    #[test]
    fn prop_apply_edit_op_json_matches_native_dispatch(
        text in "[a-zA-Z0-9\nñáé ]{0,20}",
    ) {
        let op = EditOp::InsertText {
            at: 0,
            text: text.clone(),
            marks: vec![],
        };
        let op_json = serde_json::to_string(&op).expect("serialize");

        let json_id = create_doc();
        apply_edit_op_json(json_id, &op_json).unwrap();
        let native_id = create_doc();
        apply_edit_op(native_id, op).unwrap();

        prop_assert_eq!(
            doc_text(json_id).unwrap(),
            doc_text(native_id).unwrap()
        );
        close_doc(json_id).unwrap();
        close_doc(native_id).unwrap();
    }

    /// Snapshot/restore round-trip preserves text + block count for any
    /// sequence of newline-bearing inserts.
    #[test]
    fn prop_snapshot_restore_preserves_doc_state(
        text in "[a-zA-Z\n]{0,30}",
    ) {
        let original = create_doc();
        apply_edit_op(original, EditOp::InsertText {
            at: 0, text: text.clone(), marks: vec![],
        }).unwrap();
        let snap = snapshot(original).unwrap();
        let restored = restore_from_snapshot(&snap).unwrap();
        prop_assert_eq!(doc_text(original).unwrap(), doc_text(restored).unwrap());
        prop_assert_eq!(block_count(original).unwrap(), block_count(restored).unwrap());
        close_doc(original).unwrap();
        close_doc(restored).unwrap();
    }
}
