//! Phase 4.2 RED — visual regression on canonical glyph runs.
//!
//! Each snapshot captures a lossy projection that's stable across
//! machines: glyph_id sequence + cluster_start sequence + rounded
//! x positions per run. Sub-pixel float drift between machines is
//! avoided by rounding x/width to the nearest integer.

use apalabrar_doc_model::{Block, BlockKind, Doc, EditOp};
use apalabrar_layout::{LETTER_AT_96DPI, RenderPlan, layout};

#[derive(Debug, serde::Serialize)]
struct GlyphRunsSummary {
    run_count: usize,
    runs: Vec<RunSummary>,
}

#[derive(Debug, serde::Serialize)]
struct RunSummary {
    block_index: usize,
    line_index: usize,
    font_size_px_round: i32,
    glyph_count: usize,
    glyph_ids: Vec<u16>,
    cluster_starts: Vec<u32>,
    x_px_round: Vec<i32>,
    width_px_round: Vec<i32>,
}

fn summarize(plan: &RenderPlan) -> GlyphRunsSummary {
    let runs = plan
        .glyph_runs
        .iter()
        .map(|r| RunSummary {
            block_index: r.block_index,
            line_index: r.line_index,
            font_size_px_round: r.font_size_px.round() as i32,
            glyph_count: r.glyphs.len(),
            glyph_ids: r.glyphs.iter().map(|g| g.glyph_id).collect(),
            cluster_starts: r.glyphs.iter().map(|g| g.cluster_start).collect(),
            x_px_round: r.glyphs.iter().map(|g| g.x_px.round() as i32).collect(),
            width_px_round: r.glyphs.iter().map(|g| g.width_px.round() as i32).collect(),
        })
        .collect();
    GlyphRunsSummary {
        run_count: plan.glyph_runs.len(),
        runs,
    }
}

#[test]
fn snapshot_glyphs_for_hello_world() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_glyphs_for_heading_h1() {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind: BlockKind::Heading { level: 1 },
            text: "Apalabrar".into(),
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_glyphs_for_latam_diacritics() {
    let mut d = Doc::new();
    d.insert(0, "Año cálido");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_glyphs_for_multi_paragraph() {
    let mut d = Doc::new();
    d.insert(0, "Lorem ipsum.\nDolor sit amet.");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_glyphs_for_wrapped_paragraph() {
    let mut d = Doc::new();
    let long = "lorem ipsum dolor sit amet ".repeat(20);
    d.insert(0, &long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}
