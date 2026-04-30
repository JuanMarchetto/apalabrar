//! Phase 4.5 RED — registry-backed `editor_core::find` + bridge JSON.
//!
//! The pure-string engine lives in `tests/find.rs`. These tests exercise:
//! - `apalabrar_editor_core::find` (registry wrapper around `find::find`)
//! - `apalabrar_editor_core::bridge::find_json` (JSON wrapper for wasm)

use apalabrar_editor_core::bridge::{create_doc, find_json};
use apalabrar_editor_core::find::{FindOptions, Match};
use apalabrar_editor_core::{Error, close_doc, find, open_docx};

mod common;
use common::build_minimal_docx;

// ─────────────────────────────────────────────────────────────────
// Public `editor_core::find` (registry-backed)
// ─────────────────────────────────────────────────────────────────

#[test]
fn find_on_doc_returns_codepoint_matches_in_doc_text() {
    // Build a doc with known text and search it through the registry.
    let bytes = build_minimal_docx("hello world hello");
    let id = open_docx(&bytes).unwrap();
    let matches = find(id, "hello", FindOptions::case_sensitive()).unwrap();
    assert_eq!(matches, vec![0..5, 12..17]);
    close_doc(id).unwrap();
}

#[test]
fn find_on_unknown_doc_returns_unknown_doc_error() {
    // Use a high-numbered DocId that the registry won't have.
    let bogus = open_docx(&build_minimal_docx("anything")).unwrap();
    close_doc(bogus).unwrap(); // close, then re-search → unknown
    let result = find(bogus, "anything", FindOptions::default());
    assert!(
        matches!(result, Err(Error::UnknownDoc(_))),
        "expected UnknownDoc, got {:?}",
        result
    );
}

#[test]
fn find_with_empty_needle_returns_empty_vec_not_error() {
    let bytes = build_minimal_docx("hello world");
    let id = open_docx(&bytes).unwrap();
    let matches = find(id, "", FindOptions::default()).unwrap();
    assert_eq!(matches, Vec::<Match>::new());
    close_doc(id).unwrap();
}

// ─────────────────────────────────────────────────────────────────
// `bridge::find_json` (JSON wire format for wasm)
// ─────────────────────────────────────────────────────────────────

#[test]
fn find_json_round_trips_matches_as_json_array() {
    // Use bridge::create_doc + apply_edit_op to seed text without
    // going through the OOXML path.
    use apalabrar_doc_model::EditOp;
    use apalabrar_editor_core::bridge::apply_edit_op;
    let id = create_doc();
    apply_edit_op(
        id,
        EditOp::InsertText {
            at: 0,
            text: "ab ab".into(),
            marks: vec![],
        },
    )
    .unwrap();
    let opts_json = r#"{"caseSensitive":true,"wholeWord":false}"#;
    let json = find_json(id, "ab", opts_json).unwrap();
    // Expect [{"start":0,"end":2},{"start":3,"end":5}]
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!([
            { "start": 0, "end": 2 },
            { "start": 3, "end": 5 },
        ]),
    );
    close_doc(id).unwrap();
}

#[test]
fn find_json_with_malformed_options_json_returns_json_parse_failed() {
    let id = create_doc();
    let result = find_json(id, "x", "{ this is not json");
    assert!(
        matches!(result, Err(Error::JsonParseFailed { .. })),
        "expected JsonParseFailed, got {:?}",
        result,
    );
    close_doc(id).unwrap();
}
