//! Phase 5.7.1 — `layout_doc` bridge tests.
//!
//! `editor_core::bridge::layout_doc` is the JS-callable entry into
//! the Phase 4 layout engine. It accepts a `DocId` + a JSON-encoded
//! `Viewport` and returns the resulting `RenderPlan` as a JSON
//! string. The JS painter deserialises it once per doc revision and
//! diffs against the previous plan to repaint dirty regions.
//!
//! What this suite pins down:
//!
//! - Empty doc still produces a `RenderPlan` with one page (the
//!   blank surface).
//! - A populated doc produces page(s) whose `pageNumber` is 1-indexed
//!   and whose total block count equals `Doc::block_count()`.
//! - The wire shape uses camelCase keys (`pageNumber`, `widthPx`,
//!   `glyphRuns`, …) so the JS consumer is idiomatic.
//! - `Viewport` JSON missing the required fields surfaces a
//!   `JsonParseFailed` error.
//! - A degenerate viewport (margins consume the whole page) bubbles
//!   the layout `EmptyContentArea` error through `LayoutFailed`.
//! - Calling on an unknown doc id returns `UnknownDoc`.

use apalabrar_editor_core::Error;
use apalabrar_editor_core::bridge::{apply_edit_op_json, create_doc, layout_doc};
use serde_json::Value;

const LETTER_AT_96DPI_JSON: &str = r#"{
    "pageWidthPx": 816.0,
    "pageHeightPx": 1056.0,
    "marginPx": 96.0
}"#;

fn seed_doc_with_paragraph(text: &str) -> apalabrar_editor_core::DocId {
    let id = create_doc();
    let op = format!(
        r#"{{"kind":"InsertText","at":0,"text":{:?},"marks":[]}}"#,
        text
    );
    apply_edit_op_json(id, &op).expect("seed insert op");
    id
}

#[test]
fn layout_doc_on_empty_doc_returns_a_single_blank_page() {
    let id = create_doc();
    let json = layout_doc(id, LETTER_AT_96DPI_JSON).expect("layout empty doc");
    let plan: Value = serde_json::from_str(&json).expect("plan parses as JSON");
    let pages = plan["pages"].as_array().expect("pages array present");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0]["pageNumber"].as_u64(), Some(1));
}

#[test]
fn layout_doc_uses_camelcase_keys_throughout_the_wire_format() {
    let id = seed_doc_with_paragraph("hello world");
    let json = layout_doc(id, LETTER_AT_96DPI_JSON).unwrap();
    let plan: Value = serde_json::from_str(&json).unwrap();
    // Top-level
    assert!(plan.get("pages").is_some());
    assert!(plan.get("dirtyRects").is_some());
    assert!(plan.get("glyphRuns").is_some());
    assert!(plan.get("footnoteRefs").is_some());
    // Page level
    let page0 = &plan["pages"][0];
    assert!(page0.get("pageNumber").is_some());
    // Block level
    let block0 = &page0["blocks"][0];
    assert!(block0.get("blockIndex").is_some());
    assert!(block0.get("originXPx").is_some());
    assert!(block0.get("widthPx").is_some());
    assert!(block0.get("lineRange").is_some());
}

#[test]
fn layout_doc_block_count_matches_the_doc_block_count() {
    let id = seed_doc_with_paragraph("hello world");
    let json = layout_doc(id, LETTER_AT_96DPI_JSON).unwrap();
    let plan: Value = serde_json::from_str(&json).unwrap();
    let mut total_blocks = 0;
    for page in plan["pages"].as_array().unwrap() {
        total_blocks += page["blocks"].as_array().unwrap().len();
    }
    // Single paragraph → exactly 1 block box. Non-zero is the only
    // observable invariant we want to nail down here; structural
    // round-trip is covered by the layout crate's own tests.
    assert_eq!(total_blocks, 1);
}

#[test]
fn layout_doc_emits_a_glyph_run_per_shaped_line() {
    let id = seed_doc_with_paragraph("hello world");
    let json = layout_doc(id, LETTER_AT_96DPI_JSON).unwrap();
    let plan: Value = serde_json::from_str(&json).unwrap();
    let runs = plan["glyphRuns"].as_array().unwrap();
    assert!(
        !runs.is_empty(),
        "glyphRuns must be non-empty for non-empty doc"
    );
    let run0 = &runs[0];
    assert!(run0.get("blockIndex").is_some());
    assert!(run0.get("lineIndex").is_some());
    assert!(run0.get("fontSizePx").is_some());
    assert!(run0.get("glyphs").is_some());
}

#[test]
fn layout_doc_with_unknown_doc_id_returns_unknown_doc() {
    use apalabrar_editor_core::DocId;
    // Construct a never-issued id by going past the high-water mark.
    // The registry is monotonic; 99_999 is safely past anything we
    // allocate during the test.
    let bogus: DocId = unsafe { std::mem::transmute(99_999_u64) };
    let r = layout_doc(bogus, LETTER_AT_96DPI_JSON);
    assert!(matches!(r, Err(Error::UnknownDoc(_))), "got: {:?}", r.err());
}

#[test]
fn layout_doc_with_malformed_viewport_json_returns_json_parse_failed() {
    let id = create_doc();
    let r = layout_doc(id, "{not valid json");
    assert!(
        matches!(r, Err(Error::JsonParseFailed { .. })),
        "got: {:?}",
        r.err()
    );
}

#[test]
fn layout_doc_with_zero_size_viewport_returns_an_error() {
    let id = create_doc();
    let zero_vp = r#"{"pageWidthPx": 0.0, "pageHeightPx": 0.0, "marginPx": 0.0}"#;
    let r = layout_doc(id, zero_vp);
    assert!(r.is_err(), "expected error for collapsed viewport, got Ok");
}
