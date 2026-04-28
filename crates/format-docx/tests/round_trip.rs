//! OOXML round-trip tests for Validation Gate 4.
//!
//! Coverage:
//! - 25 corpus fixtures (Phase 2.6 expansion) across 6 categories
//!   plus one synthetic edge case, each round-trips byte-equivalent
//!   after whitespace + attribute-order normalization.
//! - One modify-then-write test that asserts only `word/document.xml`
//!   changes after a paragraph edit; every other zip part (styles,
//!   settings, fontTable, etc.) is preserved verbatim.
//! - Error-path tests for empty / non-zip / non-docx input.
//! - Multibyte LATAM round-trip via the structural surface (cross-checks
//!   against parse_text from Gate 2).

use apalabrar_format_docx::{Error, read, write};

mod common;
use common::{assert_unchanged_parts_equivalent, assert_zip_xml_equivalent};

// ---------- Validation Gate 4 starter set (5 fixtures) ----------

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

// ---------- Phase 2.6 expansion (20 fixtures) ----------

const ACADEMIC_NUMBERED_LIST: &[u8] =
    include_bytes!("../../../tests-corpus/academic/numbered-list.docx");
const ACADEMIC_BLOCKQUOTE: &[u8] = include_bytes!("../../../tests-corpus/academic/blockquote.docx");
const ACADEMIC_MIXED_EMPHASIS: &[u8] =
    include_bytes!("../../../tests-corpus/academic/mixed-emphasis.docx");

const MULTILINGUAL_PORTUGUESE: &[u8] =
    include_bytes!("../../../tests-corpus/multilingual/portuguese.docx");
const MULTILINGUAL_FRENCH_GERMAN: &[u8] =
    include_bytes!("../../../tests-corpus/multilingual/french-german.docx");
const MULTILINGUAL_MIXED_SCRIPT: &[u8] =
    include_bytes!("../../../tests-corpus/multilingual/mixed-script.docx");

const TABLES_MERGED_CELLS: &[u8] = include_bytes!("../../../tests-corpus/tables/merged-cells.docx");
const TABLES_MULTI_ROW_HEADER: &[u8] =
    include_bytes!("../../../tests-corpus/tables/multi-row-header.docx");
const TABLES_NUMERIC_DATA: &[u8] = include_bytes!("../../../tests-corpus/tables/numeric-data.docx");

const FOOTNOTES_MULTIPLE: &[u8] =
    include_bytes!("../../../tests-corpus/footnotes/multiple-footnotes.docx");
const FOOTNOTE_WITH_FORMATTING: &[u8] =
    include_bytes!("../../../tests-corpus/footnotes/footnote-with-formatting.docx");
const FOOTNOTES_CROSS_PARAGRAPH: &[u8] =
    include_bytes!("../../../tests-corpus/footnotes/cross-paragraph-footnotes.docx");

const EQUATIONS_INLINE: &[u8] =
    include_bytes!("../../../tests-corpus/equations/inline-formulae.docx");
const EQUATIONS_GREEK: &[u8] = include_bytes!("../../../tests-corpus/equations/greek-symbols.docx");
const EQUATIONS_DISPLAY: &[u8] =
    include_bytes!("../../../tests-corpus/equations/display-math.docx");
const EQUATIONS_MIXED_TEXT: &[u8] =
    include_bytes!("../../../tests-corpus/equations/mixed-text.docx");

const TRACKED_INSERTION: &[u8] = include_bytes!("../../../tests-corpus/tracked/insertion.docx");
const TRACKED_DELETION: &[u8] = include_bytes!("../../../tests-corpus/tracked/deletion.docx");
const TRACKED_COMMENTS: &[u8] = include_bytes!("../../../tests-corpus/tracked/comments.docx");
const TRACKED_MIXED: &[u8] = include_bytes!("../../../tests-corpus/tracked/mixed-revisions.docx");

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

// ---------- Phase 2.6 round-trips (20 fixtures) ----------

#[test]
fn roundtrip_academic_numbered_list() {
    assert_unmodified_roundtrip("academic-numbered-list", ACADEMIC_NUMBERED_LIST);
}

#[test]
fn roundtrip_academic_blockquote() {
    assert_unmodified_roundtrip("academic-blockquote", ACADEMIC_BLOCKQUOTE);
}

#[test]
fn roundtrip_academic_mixed_emphasis() {
    assert_unmodified_roundtrip("academic-mixed-emphasis", ACADEMIC_MIXED_EMPHASIS);
}

#[test]
fn roundtrip_multilingual_portuguese() {
    assert_unmodified_roundtrip("multilingual-portuguese", MULTILINGUAL_PORTUGUESE);
}

#[test]
fn roundtrip_multilingual_french_german() {
    assert_unmodified_roundtrip("multilingual-french-german", MULTILINGUAL_FRENCH_GERMAN);
}

#[test]
fn roundtrip_multilingual_mixed_script() {
    assert_unmodified_roundtrip("multilingual-mixed-script", MULTILINGUAL_MIXED_SCRIPT);
}

#[test]
fn roundtrip_tables_merged_cells() {
    assert_unmodified_roundtrip("tables-merged-cells", TABLES_MERGED_CELLS);
}

#[test]
fn roundtrip_tables_multi_row_header() {
    assert_unmodified_roundtrip("tables-multi-row-header", TABLES_MULTI_ROW_HEADER);
}

#[test]
fn roundtrip_tables_numeric_data() {
    assert_unmodified_roundtrip("tables-numeric-data", TABLES_NUMERIC_DATA);
}

#[test]
fn roundtrip_footnotes_multiple() {
    assert_unmodified_roundtrip("footnotes-multiple", FOOTNOTES_MULTIPLE);
}

#[test]
fn roundtrip_footnote_with_formatting() {
    assert_unmodified_roundtrip("footnote-with-formatting", FOOTNOTE_WITH_FORMATTING);
}

#[test]
fn roundtrip_footnotes_cross_paragraph() {
    assert_unmodified_roundtrip("footnotes-cross-paragraph", FOOTNOTES_CROSS_PARAGRAPH);
}

#[test]
fn roundtrip_equations_inline() {
    assert_unmodified_roundtrip("equations-inline", EQUATIONS_INLINE);
}

#[test]
fn roundtrip_equations_greek_symbols() {
    assert_unmodified_roundtrip("equations-greek-symbols", EQUATIONS_GREEK);
}

#[test]
fn roundtrip_equations_display_math() {
    assert_unmodified_roundtrip("equations-display-math", EQUATIONS_DISPLAY);
}

#[test]
fn roundtrip_equations_mixed_text() {
    assert_unmodified_roundtrip("equations-mixed-text", EQUATIONS_MIXED_TEXT);
}

#[test]
fn roundtrip_tracked_insertion() {
    assert_unmodified_roundtrip("tracked-insertion", TRACKED_INSERTION);
}

#[test]
fn roundtrip_tracked_deletion() {
    assert_unmodified_roundtrip("tracked-deletion", TRACKED_DELETION);
}

#[test]
fn roundtrip_tracked_comments() {
    assert_unmodified_roundtrip("tracked-comments", TRACKED_COMMENTS);
}

#[test]
fn roundtrip_tracked_mixed_revisions() {
    assert_unmodified_roundtrip("tracked-mixed-revisions", TRACKED_MIXED);
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
