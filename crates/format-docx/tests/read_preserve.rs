//! RED-phase tests for `read_preserve` + `ShadowXml`.
//!
//! Goal: read a `.docx` into a `DocModel` WITHOUT losing any
//! unrecognized XML. Every zip part (recognized or not) must be
//! capturable as a verbatim shadow byte-for-byte.
//!
//! Three test pillars:
//!
//! 1. Per-fixture unit + snapshot tests. The summary captures part
//!    names, lengths, and indexed paragraph count for each of the 26
//!    corpus fixtures (25 categorized + 1 synthetic edge case).
//!    Reviewing the snapshot diff before `cargo insta accept` is the
//!    standard discipline.
//!
//! 2. Property tests with both random-bytes and valid-OOXML
//!    generators. The valid-OOXML path is the `serialize_text`
//!    output (any UTF-8 string with `\n` separators), which produces
//!    well-formed `.docx` zips that exercise the success branches of
//!    `read_preserve` and the consistency invariants between
//!    `read` and `read_preserve`.
//!
//! 3. Lossless modify-then-write across every corpus fixture with at
//!    least one paragraph. Mutating paragraph 0 via `DocModel`
//!    must leave every other zip part byte-equivalent on write —
//!    proving the unrecognized-XML preservation contract on real-
//!    world OOXML, not just the synthetic starter set.

use apalabrar_format_docx::{Error, read, read_preserve, serialize_text, write};
use proptest::prelude::*;
use serde::Serialize;

mod common;
use common::{assert_unchanged_parts_equivalent, unzip_parts};

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

/// Every corpus fixture with the label used in test failures and
/// snapshot file names. Sorted by category then alphabetical so a
/// failure in CI points at the right fixture without searching.
const ALL_FIXTURES: &[(&str, &[u8])] = &[
    ("academic_simple_paragraph", SIMPLE_PARAGRAPH),
    ("academic_heading_hierarchy", HEADING_HIERARCHY),
    ("academic_numbered_list", ACADEMIC_NUMBERED_LIST),
    ("academic_blockquote", ACADEMIC_BLOCKQUOTE),
    ("academic_mixed_emphasis", ACADEMIC_MIXED_EMPHASIS),
    ("multilingual_spanish_tildes", SPANISH_TILDES),
    ("multilingual_portuguese", MULTILINGUAL_PORTUGUESE),
    ("multilingual_french_german", MULTILINGUAL_FRENCH_GERMAN),
    ("multilingual_mixed_script", MULTILINGUAL_MIXED_SCRIPT),
    ("tables_simple_table", SIMPLE_TABLE),
    ("tables_merged_cells", TABLES_MERGED_CELLS),
    ("tables_multi_row_header", TABLES_MULTI_ROW_HEADER),
    ("tables_numeric_data", TABLES_NUMERIC_DATA),
    ("footnotes_single_footnote", SINGLE_FOOTNOTE),
    ("footnotes_multiple", FOOTNOTES_MULTIPLE),
    ("footnotes_with_formatting", FOOTNOTE_WITH_FORMATTING),
    ("footnotes_cross_paragraph", FOOTNOTES_CROSS_PARAGRAPH),
    ("equations_inline", EQUATIONS_INLINE),
    ("equations_greek_symbols", EQUATIONS_GREEK),
    ("equations_display_math", EQUATIONS_DISPLAY),
    ("equations_mixed_text", EQUATIONS_MIXED_TEXT),
    ("tracked_insertion", TRACKED_INSERTION),
    ("tracked_deletion", TRACKED_DELETION),
    ("tracked_comments", TRACKED_COMMENTS),
    ("tracked_mixed_revisions", TRACKED_MIXED),
    ("synthetic_empty_self_closing", EMPTY_SELF_CLOSING),
];

#[derive(Serialize)]
struct ShadowSummary {
    part_names: Vec<String>,
    part_count: usize,
    document_xml_len: usize,
    indexed_paragraph_count: usize,
}

fn shadow_summary(bytes: &[u8]) -> ShadowSummary {
    let (model, shadow) = read_preserve(bytes).expect("read_preserve");
    ShadowSummary {
        part_names: shadow
            .part_names()
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        part_count: shadow.len(),
        document_xml_len: shadow.document_xml().len(),
        indexed_paragraph_count: model.paragraph_count(),
    }
}

// ---------- Snapshots: one per fixture (26 total) ----------

#[test]
fn shadow_academic_simple_paragraph() {
    insta::assert_yaml_snapshot!(
        "shadow_academic_simple_paragraph",
        shadow_summary(SIMPLE_PARAGRAPH)
    );
}

#[test]
fn shadow_academic_heading_hierarchy() {
    insta::assert_yaml_snapshot!(
        "shadow_academic_heading_hierarchy",
        shadow_summary(HEADING_HIERARCHY)
    );
}

#[test]
fn shadow_academic_numbered_list() {
    insta::assert_yaml_snapshot!(
        "shadow_academic_numbered_list",
        shadow_summary(ACADEMIC_NUMBERED_LIST)
    );
}

#[test]
fn shadow_academic_blockquote() {
    insta::assert_yaml_snapshot!(
        "shadow_academic_blockquote",
        shadow_summary(ACADEMIC_BLOCKQUOTE)
    );
}

#[test]
fn shadow_academic_mixed_emphasis() {
    insta::assert_yaml_snapshot!(
        "shadow_academic_mixed_emphasis",
        shadow_summary(ACADEMIC_MIXED_EMPHASIS)
    );
}

#[test]
fn shadow_multilingual_spanish_tildes() {
    insta::assert_yaml_snapshot!(
        "shadow_multilingual_spanish_tildes",
        shadow_summary(SPANISH_TILDES)
    );
}

#[test]
fn shadow_multilingual_portuguese() {
    insta::assert_yaml_snapshot!(
        "shadow_multilingual_portuguese",
        shadow_summary(MULTILINGUAL_PORTUGUESE)
    );
}

#[test]
fn shadow_multilingual_french_german() {
    insta::assert_yaml_snapshot!(
        "shadow_multilingual_french_german",
        shadow_summary(MULTILINGUAL_FRENCH_GERMAN)
    );
}

#[test]
fn shadow_multilingual_mixed_script() {
    insta::assert_yaml_snapshot!(
        "shadow_multilingual_mixed_script",
        shadow_summary(MULTILINGUAL_MIXED_SCRIPT)
    );
}

#[test]
fn shadow_tables_simple_table() {
    insta::assert_yaml_snapshot!("shadow_tables_simple_table", shadow_summary(SIMPLE_TABLE));
}

#[test]
fn shadow_tables_merged_cells() {
    insta::assert_yaml_snapshot!(
        "shadow_tables_merged_cells",
        shadow_summary(TABLES_MERGED_CELLS)
    );
}

#[test]
fn shadow_tables_multi_row_header() {
    insta::assert_yaml_snapshot!(
        "shadow_tables_multi_row_header",
        shadow_summary(TABLES_MULTI_ROW_HEADER)
    );
}

#[test]
fn shadow_tables_numeric_data() {
    insta::assert_yaml_snapshot!(
        "shadow_tables_numeric_data",
        shadow_summary(TABLES_NUMERIC_DATA)
    );
}

#[test]
fn shadow_footnotes_single_footnote() {
    insta::assert_yaml_snapshot!(
        "shadow_footnotes_single_footnote",
        shadow_summary(SINGLE_FOOTNOTE)
    );
}

#[test]
fn shadow_footnotes_multiple() {
    insta::assert_yaml_snapshot!(
        "shadow_footnotes_multiple",
        shadow_summary(FOOTNOTES_MULTIPLE)
    );
}

#[test]
fn shadow_footnotes_with_formatting() {
    insta::assert_yaml_snapshot!(
        "shadow_footnotes_with_formatting",
        shadow_summary(FOOTNOTE_WITH_FORMATTING)
    );
}

#[test]
fn shadow_footnotes_cross_paragraph() {
    insta::assert_yaml_snapshot!(
        "shadow_footnotes_cross_paragraph",
        shadow_summary(FOOTNOTES_CROSS_PARAGRAPH)
    );
}

#[test]
fn shadow_equations_inline() {
    insta::assert_yaml_snapshot!("shadow_equations_inline", shadow_summary(EQUATIONS_INLINE));
}

#[test]
fn shadow_equations_greek_symbols() {
    insta::assert_yaml_snapshot!(
        "shadow_equations_greek_symbols",
        shadow_summary(EQUATIONS_GREEK)
    );
}

#[test]
fn shadow_equations_display_math() {
    insta::assert_yaml_snapshot!(
        "shadow_equations_display_math",
        shadow_summary(EQUATIONS_DISPLAY)
    );
}

#[test]
fn shadow_equations_mixed_text() {
    insta::assert_yaml_snapshot!(
        "shadow_equations_mixed_text",
        shadow_summary(EQUATIONS_MIXED_TEXT)
    );
}

#[test]
fn shadow_tracked_insertion() {
    insta::assert_yaml_snapshot!(
        "shadow_tracked_insertion",
        shadow_summary(TRACKED_INSERTION)
    );
}

#[test]
fn shadow_tracked_deletion() {
    insta::assert_yaml_snapshot!("shadow_tracked_deletion", shadow_summary(TRACKED_DELETION));
}

#[test]
fn shadow_tracked_comments() {
    insta::assert_yaml_snapshot!("shadow_tracked_comments", shadow_summary(TRACKED_COMMENTS));
}

#[test]
fn shadow_tracked_mixed_revisions() {
    insta::assert_yaml_snapshot!(
        "shadow_tracked_mixed_revisions",
        shadow_summary(TRACKED_MIXED)
    );
}

#[test]
fn shadow_synthetic_empty_self_closing() {
    insta::assert_yaml_snapshot!(
        "shadow_synthetic_empty_self_closing",
        shadow_summary(EMPTY_SELF_CLOSING)
    );
}

// ---------- Lossless: read_preserve → mutate → write ----------
//
// For every corpus fixture, mutate paragraph 0 and assert that every
// zip part EXCEPT word/document.xml is byte-equivalent (after XML
// normalization for XML/.rels parts, exact match for binaries). This
// is the structural lossless contract: anything the structural model
// doesn't interpret survives editing untouched.

#[test]
fn lossless_modify_one_paragraph_preserves_unrecognized_xml_for_every_fixture() {
    for (label, bytes) in ALL_FIXTURES {
        let (mut model, _shadow) = read_preserve(bytes).expect("read_preserve");
        assert!(
            model.paragraph_count() >= 1,
            "{label}: every corpus fixture should have at least one paragraph; \
             update this assertion if you intentionally added a 0-paragraph fixture"
        );
        model
            .set_paragraph_text(0, "MUTATED_FOR_LOSSLESS_TEST")
            .expect("set_paragraph_text");
        let output = write(&model).expect("write");
        assert_unchanged_parts_equivalent(label, bytes, &output, &["word/document.xml"]);
    }
}

#[test]
fn shadow_survives_doc_model_mutation_unchanged() {
    // The shadow is captured at read time. After arbitrary DocModel
    // mutations (set_paragraph_text → write), the shadow must still
    // expose the ORIGINAL bytes — not a view of the mutated model.
    // This is the "lossless escape hatch" property in API form.
    let (mut model, shadow) = read_preserve(SIMPLE_PARAGRAPH).expect("read_preserve");
    let original_doc_xml = shadow.document_xml().to_vec();
    let original_part_names: Vec<String> =
        shadow.part_names().into_iter().map(str::to_owned).collect();

    model
        .set_paragraph_text(0, "MUTATION_THAT_MUST_NOT_AFFECT_SHADOW")
        .expect("set_paragraph_text");
    let _ = write(&model).expect("write"); // exercise the mutation path

    // Shadow accessors return the ORIGINAL bytes regardless.
    assert_eq!(
        shadow.document_xml(),
        original_doc_xml.as_slice(),
        "shadow.document_xml() must not change after DocModel mutation"
    );
    let names_after: Vec<String> = shadow.part_names().into_iter().map(str::to_owned).collect();
    assert_eq!(
        names_after, original_part_names,
        "shadow.part_names() must not change after DocModel mutation"
    );
}

#[test]
fn shadow_part_returns_none_for_unknown_name() {
    // Negative path: querying a name the .docx didn't carry returns
    // None, not the empty slice or a panic. Catches accessor mutations
    // like swapping `.get(name)` for `.values().next()`.
    let (_model, shadow) = read_preserve(SIMPLE_PARAGRAPH).expect("read_preserve");
    assert!(shadow.part("does/not/exist.xml").is_none());
    assert!(
        shadow.part("word/document.xml").is_some(),
        "control: known part still resolves"
    );
}

#[test]
fn shadow_len_is_consistent_with_part_names() {
    // Any successful read produces a shadow whose `len()` matches the
    // count of `part_names()`. Mutating `.len` to a constant or to
    // `part_names().len() + 1` surfaces here.
    let (_model, shadow) = read_preserve(SIMPLE_PARAGRAPH).expect("read_preserve");
    assert_eq!(
        shadow.len(),
        shadow.part_names().len(),
        "shadow.len() must agree with shadow.part_names().len()"
    );
    assert!(
        shadow.len() > 0,
        "successful read_preserve must produce a non-empty shadow"
    );
}

#[test]
fn shadow_part_bytes_match_zip_entry_for_every_fixture() {
    // The shadow's part(name) must return the EXACT bytes the underlying
    // zip stored under that name. This is the lossless contract at the
    // shadow surface itself, not just at write time.
    for (label, bytes) in ALL_FIXTURES {
        let (_model, shadow) = read_preserve(bytes).expect("read_preserve");
        let zip_parts = unzip_parts(bytes);
        for (name, expected) in &zip_parts {
            let actual = shadow
                .part(name)
                .unwrap_or_else(|| panic!("{label}: shadow missing part {name}"));
            assert_eq!(
                actual, expected,
                "{label}: shadow.part({name}) bytes diverge from zip entry"
            );
        }
        // And the reverse: shadow has no extra parts beyond the zip.
        let shadow_names: std::collections::BTreeSet<_> = shadow.part_names().into_iter().collect();
        let zip_names: std::collections::BTreeSet<_> =
            zip_parts.keys().map(String::as_str).collect();
        assert_eq!(
            shadow_names, zip_names,
            "{label}: shadow part names diverge from zip"
        );
    }
}

#[test]
fn shadow_document_xml_matches_zip_entry_for_every_fixture() {
    for (label, bytes) in ALL_FIXTURES {
        let (_model, shadow) = read_preserve(bytes).expect("read_preserve");
        let zip_parts = unzip_parts(bytes);
        let zip_doc = zip_parts
            .get("word/document.xml")
            .unwrap_or_else(|| panic!("{label}: zip missing word/document.xml"));
        assert_eq!(
            shadow.document_xml(),
            zip_doc.as_slice(),
            "{label}: shadow.document_xml() diverges from zip entry"
        );
    }
}

// ---------- read vs read_preserve agreement ----------

#[test]
fn read_and_read_preserve_agree_on_paragraph_count_for_every_fixture() {
    for (label, bytes) in ALL_FIXTURES {
        let model_a = read(bytes).expect("read");
        let (model_b, _) = read_preserve(bytes).expect("read_preserve");
        assert_eq!(
            model_a.paragraph_count(),
            model_b.paragraph_count(),
            "{label}: paragraph_count diverges between read and read_preserve"
        );
    }
}

#[test]
fn read_and_read_preserve_agree_on_paragraph_text_for_every_fixture() {
    for (label, bytes) in ALL_FIXTURES {
        let model_a = read(bytes).expect("read");
        let (model_b, _) = read_preserve(bytes).expect("read_preserve");
        let n = model_a.paragraph_count();
        for i in 0..n {
            assert_eq!(
                model_a.paragraph_text(i),
                model_b.paragraph_text(i),
                "{label}: paragraph_text({i}) diverges between read and read_preserve"
            );
        }
    }
}

// ---------- Error paths ----------

#[test]
fn read_preserve_rejects_empty_bytes_with_empty_input() {
    let result = read_preserve(&[]);
    assert!(
        matches!(result, Err(Error::EmptyInput)),
        "expected EmptyInput, got {:?}",
        result.err()
    );
}

#[test]
fn read_preserve_rejects_garbage_bytes_with_invalid_zip_or_invalid_ooxml() {
    let result = read_preserve(b"this is definitely not a docx zip");
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
fn read_preserve_rejects_zip_without_document_xml() {
    // Build a tiny valid zip with a single non-document.xml entry. The
    // structural reader must reject it because word/document.xml is
    // mandatory.
    let mut buf: Vec<u8> = Vec::new();
    {
        use std::io::Write as _;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        zip.start_file("not-document.xml", zip::write::FileOptions::default())
            .expect("start_file");
        zip.write_all(b"<x/>").expect("write");
        zip.finish().expect("finish");
    }
    let result = read_preserve(&buf);
    assert!(
        matches!(result, Err(Error::InvalidOoxml(_))),
        "expected InvalidOoxml(missing word/document.xml), got {:?}",
        result.err()
    );
}

// ---------- Property tests ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// `read_preserve` must never panic — any non-OOXML or malformed
    /// byte stream produces an `Err`, mirroring the existing `read`
    /// invariant.
    #[test]
    fn prop_read_preserve_never_panics_on_random_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        let _ = read_preserve(&bytes);
    }

    /// On any input that `read_preserve` accepts, the returned
    /// `DocModel` must satisfy the basic validity invariant: every
    /// paragraph index `0..paragraph_count` resolves to `Some(_)`
    /// text. This catches any mutation that mis-counts paragraphs or
    /// off-by-ones the indexed range.
    #[test]
    fn prop_read_preserve_doc_model_indices_resolve(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        if let Ok((model, _shadow)) = read_preserve(&bytes) {
            let n = model.paragraph_count();
            for i in 0..n {
                prop_assert!(
                    model.paragraph_text(i).is_some(),
                    "paragraph_text({i}) returned None despite i < paragraph_count={n}"
                );
            }
        }
    }

    /// On any input that `read_preserve` accepts, the shadow's part
    /// count must match the actual zip entry count (no parts dropped,
    /// none invented).
    #[test]
    fn prop_read_preserve_part_count_matches_zip(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        if let Ok((_model, shadow)) = read_preserve(&bytes) {
            let zip_parts = unzip_parts(&bytes);
            prop_assert_eq!(shadow.len(), zip_parts.len());
        }
    }

    /// Valid-OOXML generator: take a small UTF-8 string, run it
    /// through `serialize_text` to produce a real `.docx` zip, then
    /// assert that `read_preserve` succeeds, the model's paragraph
    /// count matches the line count of the original string, and the
    /// shadow's `document_xml()` is non-empty.
    #[test]
    fn prop_valid_ooxml_round_trips_through_read_preserve(
        text in "[a-zA-Z0-9 \n]{0,30}",
    ) {
        let bytes = serialize_text(&text).unwrap();
        let (model, shadow) = read_preserve(&bytes).expect("read_preserve must accept valid OOXML");
        let expected_paragraphs = text.split('\n').count();
        prop_assert_eq!(model.paragraph_count(), expected_paragraphs);
        prop_assert!(
            !shadow.document_xml().is_empty(),
            "shadow.document_xml() must be non-empty for any valid OOXML"
        );
        prop_assert!(
            shadow.part("word/document.xml").is_some(),
            "shadow.part(word/document.xml) must resolve for any valid OOXML"
        );
    }

    /// `read` and `read_preserve` must agree on paragraph_count for
    /// any bytes either succeeds on. If `read_preserve` accepts the
    /// input but `read` doesn't (or vice versa), we have an internal
    /// divergence.
    #[test]
    fn prop_read_and_read_preserve_agree(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        let a = read(&bytes);
        let b = read_preserve(&bytes);
        match (a, b) {
            (Ok(model_a), Ok((model_b, _))) => {
                prop_assert_eq!(model_a.paragraph_count(), model_b.paragraph_count());
            }
            (Err(_), Err(_)) => { /* both reject — fine */ }
            (Ok(_), Err(e)) => prop_assert!(false, "read OK but read_preserve Err: {e}"),
            (Err(e), Ok(_)) => prop_assert!(false, "read Err but read_preserve OK: {e}"),
        }
    }
}
