//! RED-phase tests for `write_preserve` (Prompt 3.2 — DOCX write with
//! lossless preservation).
//!
//! Three test pillars matching the prompt:
//!
//! 1. Round-trip via `write`: `read(bytes) → write(doc) → read(again)`
//!    yields a `DocModel` equivalent to the first read on the editable
//!    surface (paragraph_count + paragraph_text per index).
//!
//! 2. Lossless via `write_preserve`: `read_preserve(bytes) →
//!    write_preserve(doc, shadow)` produces bytes that are
//!    byte-equivalent to the input on every zip part (after the same
//!    XML normalization the round-trip suite uses) when the `DocModel`
//!    is unmodified. With a single mutation, every part EXCEPT
//!    `word/document.xml` stays byte-equivalent.
//!
//! 3. Property test: arbitrary valid OOXML (generated via
//!    `serialize_text`) round-trips losslessly through
//!    `read_preserve` + `write_preserve`.

use apalabrar_format_docx::{read, read_preserve, serialize_text, write, write_preserve};
use proptest::prelude::*;

mod common;
use common::{assert_unchanged_parts_equivalent, assert_zip_xml_equivalent, unzip_parts};

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

/// Every corpus fixture with a label for failure messages.
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

// ---------- Pillar 1: round-trip via `write` ----------
//
// `read → write → read` produces a DocModel equivalent on the editable
// surface. The existing round_trip.rs suite asserts byte-equivalence,
// which is a stronger property — but the prompt explicitly asks for
// model-level round-trip, so we cover that as a separate, weaker
// assertion that surfaces issues `write` might introduce in
// re-readability that don't show up at the zip-byte level.

#[test]
fn round_trip_via_write_yields_equivalent_model_for_every_fixture() {
    for (label, bytes) in ALL_FIXTURES {
        let original = read(bytes).expect("read");
        let written = write(&original).expect("write");
        let reread = read(&written).expect("re-read");
        assert_eq!(
            original.paragraph_count(),
            reread.paragraph_count(),
            "{label}: paragraph_count differs after read→write→re-read"
        );
        for i in 0..original.paragraph_count() {
            assert_eq!(
                original.paragraph_text(i),
                reread.paragraph_text(i),
                "{label}: paragraph_text({i}) differs after read→write→re-read"
            );
        }
    }
}

#[test]
fn round_trip_via_write_after_mutation_reflects_mutation_on_re_read() {
    for (label, bytes) in ALL_FIXTURES {
        let mut model = read(bytes).expect("read");
        assert!(
            model.paragraph_count() >= 1,
            "{label}: corpus invariant — every fixture has at least one paragraph"
        );
        let original_other_texts: Vec<String> = (1..model.paragraph_count())
            .filter_map(|i| model.paragraph_text(i).map(str::to_owned))
            .collect();
        let probe = format!("MUTATION_PROBE_{label}");
        model.set_paragraph_text(0, &probe).expect("set");
        let bytes2 = write(&model).expect("write");
        let reread = read(&bytes2).expect("re-read");
        assert_eq!(
            reread.paragraph_text(0),
            Some(probe.as_str()),
            "{label}: mutated paragraph 0 must reflect on re-read"
        );
        for (j, original) in original_other_texts.iter().enumerate() {
            let i = j + 1;
            assert_eq!(
                reread.paragraph_text(i),
                Some(original.as_str()),
                "{label}: untouched paragraph {i} drifted on re-read"
            );
        }
    }
}

// ---------- Pillar 2: lossless via `write_preserve` ----------
//
// On an unmodified DocModel, `write_preserve(doc, shadow)` reproduces
// the original `.docx` bytes byte-for-byte (after XML normalization).
// On a single paragraph mutation, only `word/document.xml` should
// differ — every other zip part survives verbatim.

#[test]
fn write_preserve_unmodified_is_byte_equivalent_for_every_fixture() {
    for (label, bytes) in ALL_FIXTURES {
        let (model, shadow) = read_preserve(bytes).expect("read_preserve");
        let output = write_preserve(&model, &shadow).expect("write_preserve");
        assert_zip_xml_equivalent(label, bytes, &output);
    }
}

#[test]
fn write_preserve_after_mutation_keeps_other_parts_byte_equivalent() {
    for (label, bytes) in ALL_FIXTURES {
        let (mut model, shadow) = read_preserve(bytes).expect("read_preserve");
        assert!(
            model.paragraph_count() >= 1,
            "{label}: corpus invariant — every fixture has at least one paragraph"
        );
        model
            .set_paragraph_text(0, "WRITE_PRESERVE_MUTATION_PROBE")
            .expect("set");
        let output = write_preserve(&model, &shadow).expect("write_preserve");
        assert_unchanged_parts_equivalent(label, bytes, &output, &["word/document.xml"]);
    }
}

#[test]
fn write_preserve_uses_shadow_for_non_document_parts() {
    // Verify: the shadow's bytes for non-document parts make it
    // through unchanged. Picks one part (word/styles.xml) and asserts
    // the bytes match the shadow.part(name) we captured at read time,
    // even after a DocModel mutation. Catches a write_preserve impl
    // that accidentally pulls non-document parts from `doc.parts`
    // instead of `shadow.parts`.
    let (mut model, shadow) = read_preserve(SIMPLE_PARAGRAPH).expect("read_preserve");
    model.set_paragraph_text(0, "MUT").expect("set");
    let output = write_preserve(&model, &shadow).expect("write_preserve");
    let parts = unzip_parts(&output);
    let styles = parts.get("word/styles.xml").expect("styles part present");
    assert_eq!(
        styles.as_slice(),
        shadow.part("word/styles.xml").expect("shadow has styles"),
        "non-document parts must come verbatim from the shadow"
    );
}

#[test]
fn write_preserve_dirty_document_xml_re_reads_to_the_mutation() {
    // Splicing logic exercise: dirty paragraph → write_preserve →
    // re-read → mutated paragraph reflects the new text. Mirrors
    // round_trip's set_paragraph_text test but on the preserve path.
    let (mut model, shadow) = read_preserve(HEADING_HIERARCHY).expect("read_preserve");
    let n = model.paragraph_count();
    assert!(n >= 4);
    let original_other_texts: Vec<String> = (1..n)
        .filter_map(|i| model.paragraph_text(i).map(str::to_owned))
        .collect();
    model
        .set_paragraph_text(0, "WRITE_PRESERVE_PROBE")
        .expect("set");
    let bytes = write_preserve(&model, &shadow).expect("write_preserve");
    let reread = read(&bytes).expect("re-read");
    assert_eq!(reread.paragraph_count(), n);
    assert_eq!(reread.paragraph_text(0), Some("WRITE_PRESERVE_PROBE"));
    for (j, original) in original_other_texts.iter().enumerate() {
        let i = j + 1;
        assert_eq!(reread.paragraph_text(i), Some(original.as_str()));
    }
}

#[test]
fn write_and_write_preserve_produce_equivalent_bytes_on_unmodified_model() {
    // Both code paths should converge for an unmodified DocModel
    // (every zip part is the original bytes either way). After
    // normalization, the two outputs must compare equal.
    for (label, bytes) in ALL_FIXTURES {
        let (model, shadow) = read_preserve(bytes).expect("read_preserve");
        let via_write = write(&model).expect("write");
        let via_preserve = write_preserve(&model, &shadow).expect("write_preserve");
        assert_zip_xml_equivalent(label, &via_write, &via_preserve);
    }
}

// ---------- Property test ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// Arbitrary valid OOXML (produced by `serialize_text`) round-
    /// trips losslessly through `read_preserve` + `write_preserve`
    /// when the DocModel is not mutated.
    #[test]
    fn prop_write_preserve_round_trips_valid_ooxml(
        text in "[a-zA-Z0-9 \n]{0,30}",
    ) {
        let bytes = serialize_text(&text).unwrap();
        let (model, shadow) = read_preserve(&bytes).expect("read_preserve must accept valid OOXML");
        let output = write_preserve(&model, &shadow).expect("write_preserve must succeed");
        // Re-read both and verify the editable surface matches.
        let model_a = read(&bytes).expect("re-read original");
        let model_b = read(&output).expect("re-read output");
        prop_assert_eq!(model_a.paragraph_count(), model_b.paragraph_count());
        for i in 0..model_a.paragraph_count() {
            prop_assert_eq!(model_a.paragraph_text(i), model_b.paragraph_text(i));
        }
    }

    /// Property: a mutation through `set_paragraph_text` followed by
    /// `write_preserve` produces bytes that, when re-read, expose
    /// exactly that mutation on paragraph 0 and leave every other
    /// paragraph's text intact.
    #[test]
    fn prop_write_preserve_propagates_single_paragraph_mutation(
        text in "[a-zA-Z0-9 \n]{1,30}",
        new_text in "[a-zA-Z0-9 ]{0,40}",
    ) {
        let bytes = serialize_text(&text).unwrap();
        let (mut model, shadow) = read_preserve(&bytes).expect("read_preserve");
        // serialize_text always produces >= 1 paragraph (empty text → 1 empty para).
        prop_assert!(model.paragraph_count() >= 1);
        let original_others: Vec<String> = (1..model.paragraph_count())
            .filter_map(|i| model.paragraph_text(i).map(str::to_owned))
            .collect();
        model.set_paragraph_text(0, &new_text).expect("set");
        let out = write_preserve(&model, &shadow).expect("write_preserve");
        let reread = read(&out).expect("re-read");
        prop_assert_eq!(reread.paragraph_text(0), Some(new_text.as_str()));
        for (j, original) in original_others.iter().enumerate() {
            prop_assert_eq!(reread.paragraph_text(j + 1), Some(original.as_str()));
        }
    }
}
