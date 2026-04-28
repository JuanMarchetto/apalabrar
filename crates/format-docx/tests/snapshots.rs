//! Insta snapshot tests for the parsed `DocModel` of each corpus fixture.
//!
//! The snapshot captures a stable summary (paragraph count + per-paragraph
//! text projection). Reviewing the snapshot diff manually before
//! `cargo insta accept` is the standard Apalabrar discipline — never
//! accept a snapshot you didn't read first.

use serde::Serialize;

use apalabrar_format_docx::read;

// ---------- Validation Gate 4 starter set ----------

const SIMPLE_PARAGRAPH: &[u8] =
    include_bytes!("../../../tests-corpus/academic/simple-paragraph.docx");
const HEADING_HIERARCHY: &[u8] =
    include_bytes!("../../../tests-corpus/academic/heading-hierarchy.docx");
const SPANISH_TILDES: &[u8] =
    include_bytes!("../../../tests-corpus/multilingual/spanish-tildes.docx");
const SIMPLE_TABLE: &[u8] = include_bytes!("../../../tests-corpus/tables/simple-table.docx");
const SINGLE_FOOTNOTE: &[u8] =
    include_bytes!("../../../tests-corpus/footnotes/single-footnote.docx");

// ---------- Phase 2.6 expansion ----------

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

#[derive(Serialize)]
struct DocSummary {
    paragraph_count: usize,
    paragraphs: Vec<String>,
}

fn summary(bytes: &[u8]) -> DocSummary {
    let model = read(bytes).expect("read");
    let count = model.paragraph_count();
    let paragraphs = (0..count)
        .map(|i| model.paragraph_text(i).unwrap_or("").to_string())
        .collect();
    DocSummary {
        paragraph_count: count,
        paragraphs,
    }
}

// ---------- Starter-set snapshots ----------

#[test]
fn snapshot_simple_paragraph() {
    insta::assert_yaml_snapshot!("simple_paragraph", summary(SIMPLE_PARAGRAPH));
}

#[test]
fn snapshot_heading_hierarchy() {
    insta::assert_yaml_snapshot!("heading_hierarchy", summary(HEADING_HIERARCHY));
}

#[test]
fn snapshot_spanish_tildes() {
    insta::assert_yaml_snapshot!("spanish_tildes", summary(SPANISH_TILDES));
}

#[test]
fn snapshot_simple_table() {
    insta::assert_yaml_snapshot!("simple_table", summary(SIMPLE_TABLE));
}

#[test]
fn snapshot_single_footnote() {
    insta::assert_yaml_snapshot!("single_footnote", summary(SINGLE_FOOTNOTE));
}

// ---------- Phase 2.6 snapshots ----------

#[test]
fn snapshot_academic_numbered_list() {
    insta::assert_yaml_snapshot!("academic_numbered_list", summary(ACADEMIC_NUMBERED_LIST));
}

#[test]
fn snapshot_academic_blockquote() {
    insta::assert_yaml_snapshot!("academic_blockquote", summary(ACADEMIC_BLOCKQUOTE));
}

#[test]
fn snapshot_academic_mixed_emphasis() {
    insta::assert_yaml_snapshot!("academic_mixed_emphasis", summary(ACADEMIC_MIXED_EMPHASIS));
}

#[test]
fn snapshot_multilingual_portuguese() {
    insta::assert_yaml_snapshot!("multilingual_portuguese", summary(MULTILINGUAL_PORTUGUESE));
}

#[test]
fn snapshot_multilingual_french_german() {
    insta::assert_yaml_snapshot!(
        "multilingual_french_german",
        summary(MULTILINGUAL_FRENCH_GERMAN)
    );
}

#[test]
fn snapshot_multilingual_mixed_script() {
    insta::assert_yaml_snapshot!(
        "multilingual_mixed_script",
        summary(MULTILINGUAL_MIXED_SCRIPT)
    );
}

#[test]
fn snapshot_tables_merged_cells() {
    insta::assert_yaml_snapshot!("tables_merged_cells", summary(TABLES_MERGED_CELLS));
}

#[test]
fn snapshot_tables_multi_row_header() {
    insta::assert_yaml_snapshot!("tables_multi_row_header", summary(TABLES_MULTI_ROW_HEADER));
}

#[test]
fn snapshot_tables_numeric_data() {
    insta::assert_yaml_snapshot!("tables_numeric_data", summary(TABLES_NUMERIC_DATA));
}

#[test]
fn snapshot_footnotes_multiple() {
    insta::assert_yaml_snapshot!("footnotes_multiple", summary(FOOTNOTES_MULTIPLE));
}

#[test]
fn snapshot_footnote_with_formatting() {
    insta::assert_yaml_snapshot!(
        "footnote_with_formatting",
        summary(FOOTNOTE_WITH_FORMATTING)
    );
}

#[test]
fn snapshot_footnotes_cross_paragraph() {
    insta::assert_yaml_snapshot!(
        "footnotes_cross_paragraph",
        summary(FOOTNOTES_CROSS_PARAGRAPH)
    );
}

#[test]
fn snapshot_equations_inline() {
    insta::assert_yaml_snapshot!("equations_inline", summary(EQUATIONS_INLINE));
}

#[test]
fn snapshot_equations_greek_symbols() {
    insta::assert_yaml_snapshot!("equations_greek_symbols", summary(EQUATIONS_GREEK));
}

#[test]
fn snapshot_equations_display_math() {
    insta::assert_yaml_snapshot!("equations_display_math", summary(EQUATIONS_DISPLAY));
}

#[test]
fn snapshot_equations_mixed_text() {
    insta::assert_yaml_snapshot!("equations_mixed_text", summary(EQUATIONS_MIXED_TEXT));
}

#[test]
fn snapshot_tracked_insertion() {
    insta::assert_yaml_snapshot!("tracked_insertion", summary(TRACKED_INSERTION));
}

#[test]
fn snapshot_tracked_deletion() {
    insta::assert_yaml_snapshot!("tracked_deletion", summary(TRACKED_DELETION));
}

#[test]
fn snapshot_tracked_comments() {
    insta::assert_yaml_snapshot!("tracked_comments", summary(TRACKED_COMMENTS));
}

#[test]
fn snapshot_tracked_mixed_revisions() {
    insta::assert_yaml_snapshot!("tracked_mixed_revisions", summary(TRACKED_MIXED));
}
