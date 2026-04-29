//! Phase 4.1 RED — insta YAML snapshots over canonical render plans.
//!
//! Each snapshot pins a lossy projection (page count, per-block kind,
//! line count, rounded height/origin) rather than the full `RenderPlan`
//! to dodge sub-pixel float drift between machines. The snapshot file
//! is reviewed by hand before `cargo insta accept` — never accept a
//! snapshot you haven't read.

use apalabrar_doc_model::{Block, BlockKind as DocBlockKind, Doc, EditOp};
use apalabrar_layout::{BlockKind as LayoutBlockKind, LETTER_AT_96DPI, RenderPlan, layout};

#[derive(Debug, serde::Serialize)]
struct PlanSummary {
    page_count: usize,
    block_box_count: usize,
    dirty_rect_count: usize,
    blocks: Vec<BlockSummary>,
}

#[derive(Debug, serde::Serialize)]
struct BlockSummary {
    page_index: usize,
    block_index: usize,
    kind: String,
    line_count: usize,
    height_px_round: i32,
    origin_x_px_round: i32,
}

fn summarize(plan: &RenderPlan) -> PlanSummary {
    let mut blocks = Vec::new();
    for (page_index, page) in plan.pages.iter().enumerate() {
        for bx in &page.blocks {
            blocks.push(BlockSummary {
                page_index,
                block_index: bx.block_index,
                kind: match bx.kind {
                    LayoutBlockKind::Paragraph => "Paragraph".into(),
                    LayoutBlockKind::Heading { level } => format!("Heading-{level}"),
                    LayoutBlockKind::ListItem { indent } => format!("ListItem-{indent}"),
                },
                line_count: bx.lines.len(),
                height_px_round: bx.height_px.round() as i32,
                origin_x_px_round: bx.origin_x_px.round() as i32,
            });
        }
    }
    PlanSummary {
        page_count: plan.page_count(),
        block_box_count: plan.block_box_count(),
        dirty_rect_count: plan.dirty_rects.len(),
        blocks,
    }
}

/// Insert a block of the given kind/text at position 0 of `d`.
fn prepend(d: &mut Doc, kind: DocBlockKind, text: &str) {
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind,
            text: text.into(),
        },
    })
    .unwrap();
}

#[test]
fn snapshot_small_doc_three_kinds() {
    // Build the doc back-to-front: insert at 0 each time so the visual
    // order is heading, paragraph, list items.
    let mut d = Doc::new();
    prepend(&mut d, DocBlockKind::ListItem { indent: 0 }, "Goal two");
    prepend(&mut d, DocBlockKind::ListItem { indent: 1 }, "Sub-goal");
    prepend(&mut d, DocBlockKind::ListItem { indent: 0 }, "Goal one");
    prepend(
        &mut d,
        DocBlockKind::Paragraph,
        "A browser-native academic editor.",
    );
    prepend(&mut d, DocBlockKind::Heading { level: 1 }, "Apalabrar");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_overflow_paginates() {
    let texts: Vec<String> = (0..40).map(|i| format!("Block {i} content")).collect();
    let mut d = Doc::new();
    d.insert(0, &texts.join("\n"));
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_heading_levels_fan_out() {
    let mut d = Doc::new();
    // Build h6 → h1 by prepending; final visual order is 1..=6.
    for level in (1u8..=6).rev() {
        prepend(
            &mut d,
            DocBlockKind::Heading { level },
            &format!("Heading level {level}"),
        );
    }
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_long_paragraph_wraps() {
    let long = "lorem ipsum dolor sit amet ".repeat(40);
    let mut d = Doc::new();
    d.insert(0, &long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}

#[test]
fn snapshot_mixed_kinds_one_page() {
    let mut d = Doc::new();
    // Build: H1, paragraph, list-0, list-1, list-0, paragraph.
    prepend(&mut d, DocBlockKind::Paragraph, "Closing words.");
    prepend(&mut d, DocBlockKind::ListItem { indent: 0 }, "Third bullet");
    prepend(&mut d, DocBlockKind::ListItem { indent: 1 }, "Sub-bullet");
    prepend(&mut d, DocBlockKind::ListItem { indent: 0 }, "First bullet");
    prepend(&mut d, DocBlockKind::Paragraph, "Body intro.");
    prepend(&mut d, DocBlockKind::Heading { level: 1 }, "Apalabrar");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    insta::assert_yaml_snapshot!(summarize(&plan));
}
