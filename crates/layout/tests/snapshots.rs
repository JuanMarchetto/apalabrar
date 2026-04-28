//! YAML snapshots over small synthetic documents. Each snapshot pins:
//!
//! - page_count
//! - per-block: kind, line_count, height_px (rounded), origin_x_px (rounded)
//!
//! Snapshots are reviewed manually before `cargo insta accept` per the
//! project's TDD discipline (never accept a snapshot you didn't read).

use apalabrar_layout::{Block, BlockKind, Document, Engine, LETTER_AT_96DPI, LaidOutDocument};

/// Lossy projection of the layout result that's stable enough to snapshot
/// without sub-pixel float drift.
#[derive(Debug, serde::Serialize)]
struct Summary {
    page_count: usize,
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

fn summarize(result: &LaidOutDocument) -> Summary {
    let mut blocks = Vec::new();
    for (page_index, page) in result.pages.iter().enumerate() {
        for bx in &page.blocks {
            blocks.push(BlockSummary {
                page_index,
                block_index: bx.block_index,
                kind: match bx.kind {
                    BlockKind::Paragraph => "Paragraph".into(),
                    BlockKind::Heading { level } => format!("Heading-{level}"),
                    BlockKind::ListItem { indent } => format!("ListItem-{indent}"),
                },
                line_count: bx.lines.len(),
                height_px_round: bx.height_px.round() as i32,
                origin_x_px_round: bx.origin_x_px.round() as i32,
            });
        }
    }
    Summary {
        page_count: result.page_count(),
        blocks,
    }
}

#[test]
fn snapshot_small_doc_three_kinds() {
    let mut e = Engine::new(LETTER_AT_96DPI).unwrap();
    let doc = Document::new(vec![
        Block::Heading {
            level: 1,
            text: "Apalabrar".into(),
        },
        Block::Paragraph {
            text: "A browser-native academic editor.".into(),
        },
        Block::ListItem {
            indent: 0,
            text: "Goal one".into(),
        },
        Block::ListItem {
            indent: 1,
            text: "Sub-goal".into(),
        },
        Block::ListItem {
            indent: 0,
            text: "Goal two".into(),
        },
    ]);
    let result = e.layout(&doc);
    insta::assert_yaml_snapshot!(summarize(&result));
}

#[test]
fn snapshot_overflow_paginates() {
    let mut e = Engine::new(LETTER_AT_96DPI).unwrap();
    let blocks: Vec<Block> = (0..40)
        .map(|i| Block::Paragraph {
            text: format!("Block {i} content"),
        })
        .collect();
    let doc = Document::new(blocks);
    let result = e.layout(&doc);
    insta::assert_yaml_snapshot!(summarize(&result));
}

#[test]
fn snapshot_heading_levels_fan_out() {
    let mut e = Engine::new(LETTER_AT_96DPI).unwrap();
    let blocks: Vec<Block> = (1..=6)
        .map(|level| Block::Heading {
            level,
            text: format!("Heading level {level}"),
        })
        .collect();
    let doc = Document::new(blocks);
    let result = e.layout(&doc);
    insta::assert_yaml_snapshot!(summarize(&result));
}
