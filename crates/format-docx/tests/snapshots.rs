//! Insta snapshot tests for the parsed `DocModel` of each corpus fixture.
//!
//! The snapshot captures a stable summary (paragraph count + per-paragraph
//! text projection). Reviewing the snapshot diff manually before
//! `cargo insta accept` is the standard Apalabrar discipline — never
//! accept a snapshot you didn't read first.
//!
//! These tests fail at the SPEC's `todo!()` stubs in RED. The first
//! GREEN run will generate `tests/snapshots/*.snap.new`; review each,
//! then `cargo insta accept` to baseline.

use serde::Serialize;

use apalabrar_format_docx::read;

const SIMPLE_PARAGRAPH: &[u8] =
    include_bytes!("../../../tests-corpus/academic/simple-paragraph.docx");
const HEADING_HIERARCHY: &[u8] =
    include_bytes!("../../../tests-corpus/academic/heading-hierarchy.docx");
const SPANISH_TILDES: &[u8] =
    include_bytes!("../../../tests-corpus/multilingual/spanish-tildes.docx");
const SIMPLE_TABLE: &[u8] = include_bytes!("../../../tests-corpus/tables/simple-table.docx");
const SINGLE_FOOTNOTE: &[u8] =
    include_bytes!("../../../tests-corpus/footnotes/single-footnote.docx");

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
