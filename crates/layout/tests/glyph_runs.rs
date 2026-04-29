//! Phase 4.2 RED — `RenderPlan::glyph_runs` shape + invariants.
//!
//! Every test exercises the new field on the public surface; none of
//! them poke at cosmic-text internals.

use apalabrar_doc_model::{Block, BlockKind, Doc, EditOp};
use apalabrar_layout::{GlyphRun, LETTER_AT_96DPI, PositionedGlyph, layout};

fn paragraph_doc(text: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, text);
    d
}

fn doc_with_heading(level: u8, text: &str) -> Doc {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind: BlockKind::Heading { level },
            text: text.to_owned(),
        },
    })
    .unwrap();
    d
}

// ─────────────────────────────────────────────────────────────────
// Empty + minimal cases
// ─────────────────────────────────────────────────────────────────

#[test]
fn empty_doc_emits_no_glyph_runs() {
    // The implicit empty paragraph has no shaped glyphs — emitting
    // an empty `GlyphRun` would force the renderer to special-case
    // empty runs. Skip them at extraction time.
    let d = Doc::new();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(
        plan.glyph_runs.is_empty(),
        "expected no glyph_runs for empty doc, got {} runs",
        plan.glyph_runs.len(),
    );
}

#[test]
fn single_word_emits_one_glyph_run() {
    let d = paragraph_doc("hello");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.glyph_runs.len(), 1);
    let run = &plan.glyph_runs[0];
    assert_eq!(run.block_index, 0);
    assert_eq!(run.line_index, 0);
    assert!(
        run.glyphs.len() >= 5,
        "h-e-l-l-o → 5 glyphs minimum, got {}",
        run.glyphs.len(),
    );
}

#[test]
fn glyph_ids_are_non_zero_for_ascii() {
    // DejaVu Sans has full coverage for ASCII letters; glyph_id 0
    // (`.notdef`) only emits for missing-glyph chars.
    let d = paragraph_doc("the quick brown fox");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for run in &plan.glyph_runs {
        for g in &run.glyphs {
            assert_ne!(
                g.glyph_id, 0,
                "ASCII char should map to a real glyph, not .notdef",
            );
        }
    }
}

#[test]
fn cluster_starts_are_monotonic_within_a_run() {
    let d = paragraph_doc("monotonic clusters here");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for run in &plan.glyph_runs {
        for w in run.glyphs.windows(2) {
            assert!(
                w[0].cluster_start <= w[1].cluster_start,
                "cluster_start must not decrease within a run: {:?}",
                w,
            );
        }
    }
}

#[test]
fn cluster_end_is_at_least_cluster_start() {
    let d = paragraph_doc("end >= start invariant");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for run in &plan.glyph_runs {
        for g in &run.glyphs {
            assert!(g.cluster_end >= g.cluster_start);
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Run-per-line cardinality
// ─────────────────────────────────────────────────────────────────

#[test]
fn one_glyph_run_per_block_for_single_line_blocks() {
    let mut d = Doc::new();
    d.insert(0, "first\nsecond\nthird");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    // 3 blocks, each fitting on one line → 3 glyph runs.
    assert_eq!(plan.glyph_runs.len(), 3);
    let block_indices: Vec<usize> = plan.glyph_runs.iter().map(|r| r.block_index).collect();
    assert_eq!(block_indices, vec![0, 1, 2]);
}

#[test]
fn multi_line_paragraph_emits_one_run_per_line() {
    // ~600 chars wraps into multiple lines on a Letter content area.
    let long = "lorem ipsum dolor sit amet ".repeat(40);
    let d = paragraph_doc(&long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let line_count = plan.pages[0].blocks[0].lines.len();
    let runs_for_block_0: Vec<&GlyphRun> = plan
        .glyph_runs
        .iter()
        .filter(|r| r.block_index == 0)
        .collect();
    assert!(line_count > 1, "expected wrap, got {line_count} lines");
    assert_eq!(
        runs_for_block_0.len(),
        line_count,
        "one glyph_run per shaped line",
    );
}

#[test]
fn line_index_starts_at_zero_per_block() {
    let mut d = Doc::new();
    d.insert(0, "alpha\nbeta\ngamma");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for (block_idx, run) in plan.glyph_runs.iter().enumerate() {
        assert_eq!(run.block_index, block_idx);
        assert_eq!(
            run.line_index, 0,
            "single-line block should have line_index 0",
        );
    }
}

#[test]
fn line_index_increments_within_a_wrapped_block() {
    let long = "lorem ipsum dolor sit amet ".repeat(40);
    let d = paragraph_doc(&long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let line_indices: Vec<usize> = plan
        .glyph_runs
        .iter()
        .filter(|r| r.block_index == 0)
        .map(|r| r.line_index)
        .collect();
    assert_eq!(line_indices, (0..line_indices.len()).collect::<Vec<_>>());
}

// ─────────────────────────────────────────────────────────────────
// Geometry
// ─────────────────────────────────────────────────────────────────

#[test]
fn glyph_x_positions_are_non_decreasing_within_a_run() {
    // Pen advances left-to-right for LTR text; even with kerning
    // adjustments, the x position never goes backwards in a
    // single-script LTR paragraph.
    let d = paragraph_doc("the quick brown fox jumps");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for run in &plan.glyph_runs {
        for w in run.glyphs.windows(2) {
            assert!(
                w[0].x_px <= w[1].x_px + 0.01,
                "x must not decrease in LTR run: {:?}",
                w,
            );
        }
    }
}

#[test]
fn glyph_widths_are_non_negative() {
    let d = paragraph_doc("widths cannot be negative");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for run in &plan.glyph_runs {
        for g in &run.glyphs {
            assert!(
                g.width_px >= 0.0,
                "width_px must be >= 0, got {}",
                g.width_px,
            );
        }
    }
}

#[test]
fn font_size_px_matches_paragraph_metrics() {
    // Paragraph kind uses 14 px font (per metrics_for_kind table).
    let d = paragraph_doc("hello");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let run = &plan.glyph_runs[0];
    assert!(
        (run.font_size_px - 14.0).abs() < 0.01,
        "paragraph font_size_px should be 14, got {}",
        run.font_size_px,
    );
}

#[test]
fn font_size_px_matches_heading_metrics() {
    // Heading 1 uses 24 px font.
    let d = doc_with_heading(1, "Title");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let h1_run = plan
        .glyph_runs
        .iter()
        .find(|r| (r.font_size_px - 24.0).abs() < 0.01)
        .expect("expected an h1 run with font_size_px = 24");
    assert!(!h1_run.glyphs.is_empty());
}

// ─────────────────────────────────────────────────────────────────
// Multibyte
// ─────────────────────────────────────────────────────────────────

#[test]
fn latam_diacritics_emit_glyphs() {
    let d = paragraph_doc("Año cálido en España");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.glyph_runs.len(), 1);
    assert!(plan.glyph_runs[0].glyphs.len() >= 15);
    for g in &plan.glyph_runs[0].glyphs {
        assert_ne!(g.glyph_id, 0, "DejaVu Sans covers Spanish diacritics");
    }
}

#[test]
fn cjk_text_emits_glyphs_without_panic() {
    let d = paragraph_doc("漢字テスト");
    // We don't assert glyph_id != 0 because DejaVu Sans's CJK
    // coverage is partial; .notdef is acceptable here.
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.glyph_runs.len(), 1);
    assert!(!plan.glyph_runs[0].glyphs.is_empty());
}

#[test]
fn pua_marker_codepoints_pass_through_glyph_runs() {
    let d = paragraph_doc("text\u{E000}cite\u{E001}fn");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    // Marker codepoints are valid; layout must not panic on them.
    assert_eq!(plan.glyph_runs.len(), 1);
}

// ─────────────────────────────────────────────────────────────────
// Determinism + no-mutation
// ─────────────────────────────────────────────────────────────────

#[test]
fn glyph_runs_are_deterministic_across_calls() {
    let d = paragraph_doc("determinism check");
    let p1 = layout(&d, &LETTER_AT_96DPI).unwrap();
    let p2 = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(p1.glyph_runs, p2.glyph_runs);
}

#[test]
fn block_indices_in_glyph_runs_match_block_count() {
    let mut d = Doc::new();
    d.insert(0, "one\ntwo\nthree\nfour");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(
        !plan.glyph_runs.is_empty(),
        "4-paragraph doc should produce glyph runs",
    );
    let max_block = plan
        .glyph_runs
        .iter()
        .map(|r| r.block_index)
        .max()
        .expect("non-empty glyph_runs has a max block_index");
    assert!(max_block < d.block_count());
    assert!(
        plan.glyph_runs
            .iter()
            .all(|r| r.block_index < d.block_count()),
        "every block_index must point to a real block",
    );
}

// ─────────────────────────────────────────────────────────────────
// Hit-testing — clusters span the source text
// ─────────────────────────────────────────────────────────────────

#[test]
fn first_glyph_cluster_starts_at_zero() {
    let d = paragraph_doc("hello world");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.glyph_runs[0].glyphs[0].cluster_start, 0);
}

#[test]
fn glyph_traits_are_useful() {
    let d = paragraph_doc("hi");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let g: PositionedGlyph = plan.glyph_runs[0].glyphs[0];
    let copy = g;
    let cloned = g;
    assert_eq!(copy, cloned);
    let dbg = format!("{g:?}");
    assert!(dbg.contains("PositionedGlyph"));
    let run_dbg = format!("{:?}", plan.glyph_runs[0]);
    assert!(run_dbg.contains("GlyphRun"));
}
