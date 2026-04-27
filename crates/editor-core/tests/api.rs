//! RED-phase integration tests for the editor-core public API.
//!
//! Tests are written against `todo!()` stubs in `lib.rs` and must all fail
//! (panic at the stub) until the GREEN phase wires the implementation
//! through `apalabrar-doc-model` (Loro) and `apalabrar-format-docx` (docx-rs).
//!
//! Coverage map:
//! - Happy path: open, project text, insert at boundaries, delete a range.
//! - Edge: empty doc, no-op insert/delete, full-text delete, round-trip.
//! - Multibyte: LATAM accents preserved, multibyte char boundaries respected.
//! - Errors: empty/garbage bytes, closed-handle ops, OOB offsets, malformed
//!   ranges, offsets inside a UTF-8 codepoint.
//! - Properties: insert+delete cancellation, OOXML round-trip preserves text.

use apalabrar_editor_core::{
    DocId, EditOp, Error, apply_op, close_doc, doc_text, open_docx, to_docx,
};

mod common;
use common::build_minimal_docx;

// ---------- Happy path ----------

#[test]
fn open_docx_returns_doc_id_for_minimal_docx() {
    let bytes = build_minimal_docx("hello world");
    let id = open_docx(&bytes).expect("minimal docx should parse");
    let _: DocId = id;
}

#[test]
fn doc_text_returns_full_paragraph_text_after_open() {
    let bytes = build_minimal_docx("hello world");
    let id = open_docx(&bytes).unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello world");
}

#[test]
fn apply_op_insert_text_at_offset_zero_prepends() {
    let bytes = build_minimal_docx("world");
    let id = open_docx(&bytes).unwrap();
    apply_op(
        id,
        EditOp::InsertText {
            offset: 0,
            text: "hello ".into(),
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello world");
}

#[test]
fn apply_op_insert_text_at_doc_length_appends() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    apply_op(
        id,
        EditOp::InsertText {
            offset: 5,
            text: " world".into(),
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello world");
}

#[test]
fn apply_op_delete_range_removes_text_slice() {
    let bytes = build_minimal_docx("hello world");
    let id = open_docx(&bytes).unwrap();
    apply_op(id, EditOp::DeleteRange { start: 5, end: 11 }).unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello");
}

// ---------- Edge cases ----------

#[test]
fn open_docx_handles_empty_paragraph() {
    let bytes = build_minimal_docx("");
    let id = open_docx(&bytes).unwrap();
    assert_eq!(doc_text(id).unwrap(), "");
}

#[test]
fn apply_op_insert_empty_string_is_noop() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    apply_op(
        id,
        EditOp::InsertText {
            offset: 2,
            text: String::new(),
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello");
}

#[test]
fn apply_op_delete_empty_range_is_noop() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    apply_op(id, EditOp::DeleteRange { start: 3, end: 3 }).unwrap();
    assert_eq!(doc_text(id).unwrap(), "hello");
}

#[test]
fn apply_op_delete_full_text_clears_document() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    apply_op(id, EditOp::DeleteRange { start: 0, end: 5 }).unwrap();
    assert_eq!(doc_text(id).unwrap(), "");
}

#[test]
fn to_docx_after_no_ops_round_trip_preserves_text() {
    let bytes = build_minimal_docx("hello world");
    let id = open_docx(&bytes).unwrap();
    let serialized = to_docx(id).unwrap();
    assert!(
        !serialized.is_empty(),
        "to_docx must produce non-empty bytes"
    );
    let id2 = open_docx(&serialized).unwrap();
    assert_eq!(doc_text(id2).unwrap(), "hello world");
}

#[test]
fn to_docx_after_insert_round_trip_preserves_edited_text() {
    let bytes = build_minimal_docx("world");
    let id = open_docx(&bytes).unwrap();
    apply_op(
        id,
        EditOp::InsertText {
            offset: 0,
            text: "hello ".into(),
        },
    )
    .unwrap();
    let serialized = to_docx(id).unwrap();
    let id2 = open_docx(&serialized).unwrap();
    assert_eq!(doc_text(id2).unwrap(), "hello world");
}

// ---------- Multibyte / LATAM ----------

#[test]
fn doc_text_preserves_latam_accented_characters() {
    let bytes = build_minimal_docx("Año nuevo, mañana");
    let id = open_docx(&bytes).unwrap();
    assert_eq!(doc_text(id).unwrap(), "Año nuevo, mañana");
}

#[test]
fn apply_op_inserts_multibyte_at_byte_offset_aligned_with_char_boundary() {
    // "café" = c(1) a(1) f(1) é(2) — 5 UTF-8 bytes, 4 chars.
    let bytes = build_minimal_docx("café");
    let id = open_docx(&bytes).unwrap();
    apply_op(
        id,
        EditOp::InsertText {
            offset: 5,
            text: "!".into(),
        },
    )
    .unwrap();
    assert_eq!(doc_text(id).unwrap(), "café!");
}

// ---------- Error cases ----------

#[test]
fn open_docx_rejects_empty_bytes_with_empty_input_error() {
    let result = open_docx(&[]);
    assert!(
        matches!(result, Err(Error::EmptyInput)),
        "expected EmptyInput, got {:?}",
        result.err()
    );
}

#[test]
fn open_docx_rejects_garbage_bytes_with_invalid_ooxml_error() {
    let garbage = b"this is not a docx file at all";
    let result = open_docx(garbage);
    assert!(
        matches!(result, Err(Error::InvalidOoxml)),
        "expected InvalidOoxml, got {:?}",
        result.err()
    );
}

#[test]
fn apply_op_after_close_returns_unknown_doc_error() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    close_doc(id).unwrap();
    let result = apply_op(
        id,
        EditOp::InsertText {
            offset: 0,
            text: "x".into(),
        },
    );
    assert!(
        matches!(result, Err(Error::UnknownDoc(_))),
        "expected UnknownDoc, got {:?}",
        result.err()
    );
}

#[test]
fn doc_text_after_close_returns_unknown_doc_error() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    close_doc(id).unwrap();
    let result = doc_text(id);
    assert!(
        matches!(result, Err(Error::UnknownDoc(_))),
        "expected UnknownDoc, got {:?}",
        result.err()
    );
}

#[test]
fn close_doc_on_already_closed_returns_unknown_doc_error() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    close_doc(id).unwrap();
    let result = close_doc(id);
    assert!(
        matches!(result, Err(Error::UnknownDoc(_))),
        "expected UnknownDoc, got {:?}",
        result.err()
    );
}

#[test]
fn apply_op_insert_offset_past_doc_length_errors() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    let result = apply_op(
        id,
        EditOp::InsertText {
            offset: 999,
            text: "x".into(),
        },
    );
    assert!(
        matches!(
            result,
            Err(Error::OffsetOutOfBounds {
                offset: 999,
                len: 5
            })
        ),
        "expected OffsetOutOfBounds {{ offset: 999, len: 5 }}, got {:?}",
        result.err()
    );
}

#[test]
fn apply_op_insert_offset_inside_multibyte_char_errors() {
    // "café" = 5 bytes; offset 4 lands inside the 2-byte 'é'.
    let bytes = build_minimal_docx("café");
    let id = open_docx(&bytes).unwrap();
    let result = apply_op(
        id,
        EditOp::InsertText {
            offset: 4,
            text: "!".into(),
        },
    );
    assert!(
        matches!(result, Err(Error::OffsetNotOnCharBoundary { offset: 4 })),
        "expected OffsetNotOnCharBoundary {{ offset: 4 }}, got {:?}",
        result.err()
    );
}

#[test]
fn apply_op_delete_range_with_start_greater_than_end_errors() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    let result = apply_op(id, EditOp::DeleteRange { start: 4, end: 1 });
    assert!(
        matches!(
            result,
            Err(Error::InvalidRange {
                start: 4,
                end: 1,
                ..
            })
        ),
        "expected InvalidRange {{ start: 4, end: 1, .. }}, got {:?}",
        result.err()
    );
}

#[test]
fn apply_op_delete_range_past_doc_length_errors() {
    let bytes = build_minimal_docx("hello");
    let id = open_docx(&bytes).unwrap();
    let result = apply_op(id, EditOp::DeleteRange { start: 0, end: 999 });
    assert!(
        matches!(
            result,
            Err(Error::InvalidRange {
                start: 0,
                end: 999,
                len: 5
            })
        ),
        "expected InvalidRange {{ start: 0, end: 999, len: 5 }}, got {:?}",
        result.err()
    );
}

// ---------- Property tests ----------

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// Inserting `text` at `offset` and then deleting the byte range
    /// `[offset, offset + text.len())` must restore the original text.
    /// This is the cancellation invariant on the InsertText/DeleteRange pair.
    #[test]
    fn prop_insert_then_delete_at_same_position_returns_original_text(
        seed in "[a-z ]{0,10}",
        text in "[a-z]{1,5}",
        offset_choice in 0usize..=10,
    ) {
        let bytes = build_minimal_docx(&seed);
        let id = open_docx(&bytes).unwrap();
        let len = doc_text(id).unwrap().len();
        let offset = offset_choice.min(len);
        apply_op(id, EditOp::InsertText { offset, text: text.clone() }).unwrap();
        apply_op(
            id,
            EditOp::DeleteRange { start: offset, end: offset + text.len() },
        ).unwrap();
        prop_assert_eq!(doc_text(id).unwrap(), seed);
    }

    /// Serializing then re-opening the document preserves the plain-text
    /// projection. This is the OOXML round-trip invariant for unedited docs.
    #[test]
    fn prop_serialize_then_open_preserves_doc_text(
        seed in "[a-zA-Z0-9 ]{0,30}",
    ) {
        let bytes = build_minimal_docx(&seed);
        let id = open_docx(&bytes).unwrap();
        let serialized = to_docx(id).unwrap();
        let id2 = open_docx(&serialized).unwrap();
        prop_assert_eq!(doc_text(id2).unwrap(), seed);
    }
}
