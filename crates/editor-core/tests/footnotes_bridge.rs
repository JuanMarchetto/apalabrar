//! Phase 5.1 RED — bridge JSON for footnotes_json.
//!
//! Separate integration-test binary so the `todo!()` panic in
//! `footnotes_json` doesn't poison the registry mutex shared with
//! `tests/bridge.rs` and other Phase-bridge tests.

use apalabrar_doc_model::{Block, BlockKind, BlockTree, EditOp};
use apalabrar_editor_core::bridge::{apply_edit_op, create_doc, footnotes_json};
use apalabrar_editor_core::{Error, close_doc};

fn paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph,
        text: text.into(),
    }
}

#[test]
fn footnotes_json_returns_array_in_body_position_order() {
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "abc def".into(),
            marks: vec![],
        },
    )
    .unwrap();
    // Insert footnote A at the END of body — body-order: 1st
    apply_edit_op(
        id,
        EditOp::InsertFootnote {
            at: 7,
            body: BlockTree {
                blocks: vec![paragraph("A-body")],
            },
        },
    )
    .unwrap();
    // Insert footnote B at the START — body-order: 0th
    apply_edit_op(
        id,
        EditOp::InsertFootnote {
            at: 0,
            body: BlockTree {
                blocks: vec![paragraph("B-body")],
            },
        },
    )
    .unwrap();
    let json = footnotes_json(id).unwrap();
    let arr: serde_json::Value = serde_json::from_str(&json).unwrap();
    let arr = arr.as_array().expect("footnotes_json must be a JSON array");
    assert_eq!(arr.len(), 2);
    // First entry must carry the body-text of the marker that sits
    // FIRST in the body — that's footnote B (inserted at pos 0).
    let first_body_text = arr[0]["body"]["blocks"][0]["text"]
        .as_str()
        .expect("body[0].text");
    assert_eq!(first_body_text, "B-body");
    let second_body_text = arr[1]["body"]["blocks"][0]["text"]
        .as_str()
        .expect("body[1].text");
    assert_eq!(second_body_text, "A-body");
    close_doc(id).unwrap();
}

#[test]
fn footnotes_json_returns_empty_array_for_doc_without_footnotes() {
    let id = create_doc();
    let json = footnotes_json(id).unwrap();
    assert_eq!(json, "[]");
    close_doc(id).unwrap();
}

#[test]
fn footnotes_json_on_unknown_doc_returns_unknown_doc_error() {
    let id = create_doc();
    close_doc(id).unwrap();
    let err = footnotes_json(id).unwrap_err();
    assert!(matches!(err, Error::UnknownDoc(_)));
}
