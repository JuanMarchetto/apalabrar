//! RED-phase OOXML round-trip tests for Validation Gate 4.
//!
//! Coverage:
//! - 5 corpus fixtures, each round-trips byte-equivalent after whitespace +
//!   attribute-order normalization.
//! - One modify-then-write test that asserts only `word/document.xml`
//!   changes after a paragraph edit; every other zip part (styles,
//!   settings, fontTable, etc.) is preserved verbatim.
//! - Error-path tests for empty / non-zip / non-docx input.
//! - Multibyte LATAM round-trip via the structural surface (cross-checks
//!   against parse_text from Gate 2).
//!
//! Tests fail at `todo!()` in `read` / `write` until the GREEN phase
//! wires the zip + quick-xml shadow-store implementation.

use apalabrar_format_docx::{Error, read, write};

mod common;
use common::{assert_unchanged_parts_equivalent, assert_zip_xml_equivalent};

const SIMPLE_PARAGRAPH: &[u8] =
    include_bytes!("../../../tests-corpus/academic/simple-paragraph.docx");
const HEADING_HIERARCHY: &[u8] =
    include_bytes!("../../../tests-corpus/academic/heading-hierarchy.docx");
const SPANISH_TILDES: &[u8] =
    include_bytes!("../../../tests-corpus/multilingual/spanish-tildes.docx");
const SIMPLE_TABLE: &[u8] = include_bytes!("../../../tests-corpus/tables/simple-table.docx");
const SINGLE_FOOTNOTE: &[u8] =
    include_bytes!("../../../tests-corpus/footnotes/single-footnote.docx");
const EMPTY_SELF_CLOSING: &[u8] =
    include_bytes!("../../../tests-corpus/synthetic/empty-self-closing-paragraph.docx");

fn assert_unmodified_roundtrip(label: &str, fixture: &[u8]) {
    let model = read(fixture).expect("read");
    let output = write(&model).expect("write");
    assert_zip_xml_equivalent(label, fixture, &output);
}

// ---------- 5 fixture round-trips ----------

#[test]
fn roundtrip_simple_paragraph_is_byte_equivalent_after_normalization() {
    assert_unmodified_roundtrip("simple-paragraph", SIMPLE_PARAGRAPH);
}

#[test]
fn roundtrip_heading_hierarchy_is_byte_equivalent_after_normalization() {
    assert_unmodified_roundtrip("heading-hierarchy", HEADING_HIERARCHY);
}

#[test]
fn roundtrip_spanish_tildes_is_byte_equivalent_after_normalization() {
    assert_unmodified_roundtrip("spanish-tildes", SPANISH_TILDES);
}

#[test]
fn roundtrip_simple_table_is_byte_equivalent_after_normalization() {
    assert_unmodified_roundtrip("simple-table", SIMPLE_TABLE);
}

#[test]
fn roundtrip_single_footnote_is_byte_equivalent_after_normalization() {
    assert_unmodified_roundtrip("single-footnote", SINGLE_FOOTNOTE);
}

// ---------- Lossless preservation under modification ----------

#[test]
fn read_then_modify_then_write_preserves_unchanged_xml() {
    let mut model = read(SIMPLE_PARAGRAPH).expect("read");
    assert!(
        model.paragraph_count() >= 1,
        "fixture must have at least one paragraph"
    );
    model
        .set_paragraph_text(0, "MODIFIED CONTENT")
        .expect("set_paragraph_text");
    let output = write(&model).expect("write");
    // The modified paragraph lives in word/document.xml. Every other
    // OOXML part (styles, settings, footer, fontTable, etc.) and every
    // binary part must be byte-equivalent after normalization.
    assert_unchanged_parts_equivalent(
        "modify-preserves",
        SIMPLE_PARAGRAPH,
        &output,
        &["word/document.xml"],
    );
}

// ---------- Error paths ----------

#[test]
fn read_rejects_empty_bytes_with_empty_input() {
    let result = read(&[]);
    assert!(
        matches!(result, Err(Error::EmptyInput)),
        "expected EmptyInput, got {:?}",
        result.err()
    );
}

#[test]
fn read_rejects_garbage_bytes_with_invalid_zip_or_invalid_ooxml() {
    let result = read(b"this is definitely not a docx zip");
    assert!(
        matches!(
            result,
            Err(Error::InvalidZip(_)) | Err(Error::InvalidOoxml(_))
        ),
        "expected InvalidZip or InvalidOoxml, got {:?}",
        result.err()
    );
}

#[test]
fn set_paragraph_text_out_of_bounds_returns_invalid_paragraph_index() {
    let mut model = read(SIMPLE_PARAGRAPH).expect("read");
    let len = model.paragraph_count();
    let result = model.set_paragraph_text(len + 100, "won't apply");
    assert!(
        matches!(result, Err(Error::InvalidParagraphIndex { .. })),
        "expected InvalidParagraphIndex, got {:?}",
        result.err()
    );
}

// ---------- Cross-check: structural surface preserves multibyte ----------

#[test]
fn paragraph_text_preserves_latam_accents() {
    let model = read(SPANISH_TILDES).expect("read");
    let n = model.paragraph_count();
    assert!(n >= 1, "fixture has at least one paragraph");
    let combined: String = (0..n)
        .filter_map(|i| model.paragraph_text(i))
        .collect::<Vec<_>>()
        .join(" ");
    for needle in ['ñ', 'á', 'é', 'í', 'ó', 'ú', 'ü', 'ç', '¿', '¡', 'Á'] {
        assert!(
            combined.contains(needle),
            "expected '{needle}' in parsed text, got: {combined:?}",
        );
    }
}

// ---------- Mutation-kill tests ----------
//
// Pinned exact paragraph counts per fixture so any mutation in
// `index_paragraphs` that mis-identifies <w:p> boundaries (replace == with
// !=, replace && with ||, drop the Event::End arm, etc.) flips the count
// and fails the test. The numbers come from the insta snapshots accepted
// in the GREEN commit.

#[test]
fn paragraph_count_matches_expected_per_fixture() {
    for (label, bytes, expected) in [
        ("simple-paragraph", SIMPLE_PARAGRAPH, 1usize),
        ("heading-hierarchy", HEADING_HIERARCHY, 11),
        ("spanish-tildes", SPANISH_TILDES, 7),
        ("simple-table", SIMPLE_TABLE, 11),
        ("single-footnote", SINGLE_FOOTNOTE, 1),
    ] {
        let model = read(bytes).expect("read");
        assert_eq!(
            model.paragraph_count(),
            expected,
            "{label}: paragraph count drift",
        );
    }
}

#[test]
fn read_modify_write_then_re_read_observes_modification() {
    // End-to-end exercise of the dirty-write path: rebuild_document_xml,
    // format_paragraph, and xml_escape all run when a paragraph is set.
    // Re-reading the produced bytes must reflect the new text on the
    // mutated paragraph and leave every other paragraph's text intact.
    let mut model = read(HEADING_HIERARCHY).expect("read");
    let original_count = model.paragraph_count();
    assert!(original_count >= 4, "need at least four paragraphs");

    let original_other_texts: Vec<String> = (1..original_count)
        .filter_map(|i| model.paragraph_text(i).map(str::to_owned))
        .collect();

    model
        .set_paragraph_text(0, "MUTATION_KILL_PROBE")
        .expect("set_paragraph_text");
    let bytes = write(&model).expect("write");
    let reloaded = read(&bytes).expect("re-read");
    assert_eq!(reloaded.paragraph_count(), original_count);
    assert_eq!(
        reloaded.paragraph_text(0),
        Some("MUTATION_KILL_PROBE"),
        "modified paragraph must reflect the new text after a write/re-read cycle",
    );
    for (i, original) in original_other_texts.iter().enumerate() {
        let i = i + 1;
        assert_eq!(
            reloaded.paragraph_text(i),
            Some(original.as_str()),
            "untouched paragraph {i} must keep its original text",
        );
    }
}

#[test]
fn set_paragraph_text_escapes_xml_special_characters() {
    // The dirty-write path runs xml_escape on the new text. If escaping is
    // dropped, '<', '&', '>' would either crash quick-xml on re-read or
    // surface as literal characters that change the semantic content.
    let mut model = read(SIMPLE_PARAGRAPH).expect("read");
    model
        .set_paragraph_text(0, "less< amp& greater> all together")
        .expect("set");
    let bytes = write(&model).expect("write");
    let reloaded = read(&bytes).expect("re-read");
    assert_eq!(
        reloaded.paragraph_text(0),
        Some("less< amp& greater> all together"),
        "XML special chars must round-trip through the escape/unescape pair",
    );
}

#[test]
fn set_paragraph_text_with_quotes_round_trips() {
    let mut model = read(SIMPLE_PARAGRAPH).expect("read");
    model
        .set_paragraph_text(0, r#"double "quotes" and 'apostrophes'"#)
        .expect("set");
    let bytes = write(&model).expect("write");
    let reloaded = read(&bytes).expect("re-read");
    assert_eq!(
        reloaded.paragraph_text(0),
        Some(r#"double "quotes" and 'apostrophes'"#),
    );
}

// ---------- Self-closing <w:p/> handling ----------
//
// The 5 corpus fixtures all come from LibreOffice, which always emits
// explicit `<w:p>...</w:p>` even for empty paragraphs. The OOXML schema
// nevertheless permits self-closing `<w:p/>`, and `index_paragraphs` has
// a dedicated Event::Empty arm to handle it. Without a fixture that
// exercises the arm, deleting it goes unnoticed by mutation testing.
//
// `tests-corpus/synthetic/empty-self-closing-paragraph.docx` is built
// from `simple-paragraph.docx` with a `<w:p/>` spliced in right before
// `</w:body>` — see `/tmp/make_self_closing.py` (kept out-of-tree; the
// fixture itself is the canonical artifact).

#[test]
fn read_indexes_self_closing_empty_paragraph() {
    let model = read(EMPTY_SELF_CLOSING).expect("read");
    assert_eq!(
        model.paragraph_count(),
        2,
        "synthetic fixture must surface BOTH paragraphs (the original \
         BodyText one + the self-closing empty one)",
    );
    let original_text = model.paragraph_text(0).expect("p[0] text");
    assert!(
        original_text.starts_with("This is a single paragraph fixture"),
        "p[0] should be the original BodyText paragraph; got {original_text:?}",
    );
    assert_eq!(
        model.paragraph_text(1),
        Some(""),
        "p[1] should be the empty self-closing paragraph",
    );
}

#[test]
fn roundtrip_self_closing_empty_paragraph_is_byte_equivalent() {
    assert_unmodified_roundtrip("empty-self-closing", EMPTY_SELF_CLOSING);
}
