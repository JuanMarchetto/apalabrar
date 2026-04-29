//! Phase 4.1 RED — unit tests for `apalabrar_layout::layout()`.
//!
//! Every test exercises the new free-fn surface: build a `Doc` via the
//! `apalabrar-doc-model` public API, call `layout(&doc, &vp)`, assert
//! against `RenderPlan` shape. None of the legacy `Engine`/`Document`
//! types appear here on purpose — the new surface stands on its own.

use apalabrar_doc_model::{Block, BlockKind, Doc, EditOp};
use apalabrar_layout::{Error, LETTER_AT_96DPI, Rect, RenderPlan, Viewport, layout};

// ─────────────────────────────────────────────────────────────────
// Construction helpers
// ─────────────────────────────────────────────────────────────────

/// Empty Doc — by doc-model invariant has exactly one implicit empty
/// paragraph.
fn empty_doc() -> Doc {
    Doc::new()
}

/// Single-paragraph Doc carrying `text`.
fn paragraph_doc(text: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, text);
    d
}

/// Multi-paragraph Doc — each entry becomes its own block, separated
/// by `\n` (which is doc-model's block boundary).
fn paragraphs(texts: &[&str]) -> Doc {
    let mut d = Doc::new();
    d.insert(0, &texts.join("\n"));
    d
}

/// Insert a heading with the given level via apply_edit_op. Caller
/// passes `at` — for a fresh Doc, `at = 0` puts the heading at the
/// start.
fn doc_with_heading(level: u8, text: &str) -> Doc {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind: BlockKind::Heading { level },
            text: text.to_owned(),
        },
    })
    .expect("InsertBlock heading should succeed");
    d
}

/// Insert a list item with the given indent.
fn doc_with_list_item(indent: u8, text: &str) -> Doc {
    let mut d = Doc::new();
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind: BlockKind::ListItem { indent },
            text: text.to_owned(),
        },
    })
    .expect("InsertBlock list-item should succeed");
    d
}

// ─────────────────────────────────────────────────────────────────
// Empty + minimal-doc cases
// ─────────────────────────────────────────────────────────────────

#[test]
fn empty_doc_lays_out_to_a_single_page() {
    let d = empty_doc();
    let plan = layout(&d, &LETTER_AT_96DPI).expect("layout should succeed");
    assert_eq!(
        plan.page_count(),
        1,
        "empty doc has the implicit empty paragraph"
    );
}

#[test]
fn empty_doc_block_box_count_matches_block_count() {
    let d = empty_doc();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn empty_paragraph_still_occupies_a_box_with_positive_height() {
    let d = empty_doc();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let bbox = &plan.pages[0].blocks[0];
    assert!(
        bbox.height_px > 0.0,
        "empty paragraph should still contribute a visible line height; got {}",
        bbox.height_px,
    );
}

#[test]
fn single_paragraph_lays_out_to_one_page() {
    let d = paragraph_doc("The quick brown fox jumps over the lazy dog.");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.page_count(), 1);
    assert_eq!(plan.block_box_count(), 1);
}

#[test]
fn single_paragraph_kind_is_paragraph() {
    let d = paragraph_doc("hi");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    assert_eq!(plan.pages[0].blocks[0].kind, LK::Paragraph);
}

// ─────────────────────────────────────────────────────────────────
// Heading kinds + level clamping
// ─────────────────────────────────────────────────────────────────

#[test]
fn heading_level_1_preserved() {
    let d = doc_with_heading(1, "Title");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    let kinds: Vec<_> = plan.pages[0].blocks.iter().map(|b| b.kind).collect();
    assert!(
        kinds.contains(&LK::Heading { level: 1 }),
        "expected Heading{{1}} among kinds, got {kinds:?}",
    );
}

#[test]
fn heading_level_2_preserved() {
    let d = doc_with_heading(2, "Section");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    assert!(
        plan.pages[0]
            .blocks
            .iter()
            .any(|b| b.kind == LK::Heading { level: 2 })
    );
}

#[test]
fn heading_level_3_preserved() {
    let d = doc_with_heading(3, "Sub");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    assert!(
        plan.pages[0]
            .blocks
            .iter()
            .any(|b| b.kind == LK::Heading { level: 3 })
    );
}

#[test]
fn heading_level_6_preserved() {
    let d = doc_with_heading(6, "Deepest");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    assert!(
        plan.pages[0]
            .blocks
            .iter()
            .any(|b| b.kind == LK::Heading { level: 6 })
    );
}

#[test]
fn heading_level_7_clamps_to_6() {
    // doc-model clamps on insert (Phase B), so level 7 lands as level 6
    // in the model. Layout must surface what the model says, not the
    // original input.
    let d = doc_with_heading(7, "Past max");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    let saw_h7 = plan.pages[0]
        .blocks
        .iter()
        .any(|b| matches!(b.kind, LK::Heading { level } if level == 7));
    assert!(!saw_h7, "level 7 should be clamped before layout sees it");
}

#[test]
fn heading_taller_than_paragraph() {
    // Heading uses a bigger font + larger top/bottom spacing per the
    // engine's metrics table. Sanity check the geometry.
    let h = doc_with_heading(1, "Title");
    let p = paragraph_doc("Title");
    let h_plan = layout(&h, &LETTER_AT_96DPI).unwrap();
    let p_plan = layout(&p, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    let h_height = h_plan.pages[0]
        .blocks
        .iter()
        .find(|b| matches!(b.kind, LK::Heading { level: 1 }))
        .expect("must have an h1 box")
        .height_px;
    let p_height = p_plan.pages[0].blocks[0].height_px;
    assert!(
        h_height > p_height,
        "h1 ({h_height}px) should be taller than paragraph ({p_height}px) for the same text",
    );
}

// ─────────────────────────────────────────────────────────────────
// List items + indent
// ─────────────────────────────────────────────────────────────────

#[test]
fn list_item_indent_0_preserved() {
    let d = doc_with_list_item(0, "first");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    assert!(
        plan.pages[0]
            .blocks
            .iter()
            .any(|b| b.kind == LK::ListItem { indent: 0 })
    );
}

#[test]
fn list_item_indent_2_preserved() {
    let d = doc_with_list_item(2, "deeper");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    assert!(
        plan.pages[0]
            .blocks
            .iter()
            .any(|b| b.kind == LK::ListItem { indent: 2 })
    );
}

#[test]
fn list_item_block_width_equals_content_width_minus_inset() {
    // The block's drawable area is `content_width - left_inset`. A
    // mutation that flips the sign or the operator (e.g. `+` instead
    // of `-`, `/` instead of `-`) would yield a width that's either
    // bigger than the content area or wildly off — both observable.
    let d = doc_with_list_item(2, "x");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    let bx = plan.pages[0]
        .blocks
        .iter()
        .find(|b| matches!(b.kind, LK::ListItem { indent: 2 }))
        .expect("must have a list-item box");
    let cw = LETTER_AT_96DPI.content_width();
    let expected_inset = 24.0 + 16.0 * 2.0; // = 56.0
    let expected_width = cw - expected_inset;
    assert!(
        (bx.origin_x_px - expected_inset).abs() < 0.01,
        "list-item origin_x should be {}, got {}",
        expected_inset,
        bx.origin_x_px,
    );
    assert!(
        (bx.width_px - expected_width).abs() < 0.01,
        "list-item width should be {}, got {}",
        expected_width,
        bx.width_px,
    );
}

#[test]
fn list_item_long_text_wraps_inside_inset_not_full_content_width() {
    // The wrap-width passed to cosmic-text is `content_width - inset`,
    // not `content_width`. A mutation that flips the sign would let
    // text run past the inset boundary. We assert that line widths for
    // an indented list item never exceed the printable area inside the
    // indent (content_width - inset + slop).
    let long = "lorem ipsum dolor sit amet ".repeat(30);
    let d = doc_with_list_item(3, &long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    let bx = plan.pages[0]
        .blocks
        .iter()
        .find(|b| matches!(b.kind, LK::ListItem { indent: 3 }))
        .expect("must have an indent-3 list-item box");
    assert!(bx.lines.len() > 1, "long list item should wrap");
    let inset = 24.0 + 16.0 * 3.0;
    let max_line_w = LETTER_AT_96DPI.content_width() - inset;
    for line in &bx.lines {
        assert!(
            line.width_px <= max_line_w + 1.0,
            "line.width_px = {} exceeds (content_width - inset) = {}",
            line.width_px,
            max_line_w,
        );
    }
}

#[test]
fn list_item_origin_x_grows_with_indent() {
    let i0 = doc_with_list_item(0, "x");
    let i3 = doc_with_list_item(3, "x");
    let p0 = layout(&i0, &LETTER_AT_96DPI).unwrap();
    let p3 = layout(&i3, &LETTER_AT_96DPI).unwrap();
    use apalabrar_layout::BlockKind as LK;
    let x0 = p0.pages[0]
        .blocks
        .iter()
        .find(|b| matches!(b.kind, LK::ListItem { .. }))
        .unwrap()
        .origin_x_px;
    let x3 = p3.pages[0]
        .blocks
        .iter()
        .find(|b| matches!(b.kind, LK::ListItem { .. }))
        .unwrap()
        .origin_x_px;
    assert!(
        x3 > x0,
        "indent 3 should sit further right than indent 0 ({x3} vs {x0})"
    );
}

// ─────────────────────────────────────────────────────────────────
// Multi-block flow
// ─────────────────────────────────────────────────────────────────

#[test]
fn multi_paragraph_block_box_count_equals_block_count() {
    let d = paragraphs(&["one", "two", "three", "four"]);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), d.block_count());
    assert_eq!(plan.block_box_count(), 4);
}

#[test]
fn multi_paragraph_block_indices_are_unique_and_in_order() {
    let d = paragraphs(&["one", "two", "three"]);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let indices: Vec<usize> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter().map(|b| b.block_index))
        .collect();
    assert_eq!(indices, vec![0, 1, 2]);
}

#[test]
fn block_y_origins_are_monotonic_within_a_page() {
    let d = paragraphs(&["one", "two", "three"]);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    for page in &plan.pages {
        let ys: Vec<f32> = page.blocks.iter().map(|b| b.origin_y_px).collect();
        for w in ys.windows(2) {
            assert!(w[0] < w[1], "block y origins should increase: {ys:?}");
        }
    }
}

#[test]
fn many_paragraphs_split_across_pages() {
    let texts: Vec<String> = (0..200).map(|i| format!("paragraph number {i}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let d = paragraphs(&refs);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(
        plan.page_count() >= 2,
        "200 short paragraphs should not fit on one Letter page; got {}",
        plan.page_count(),
    );
    assert_eq!(plan.block_box_count(), d.block_count());
}

#[test]
fn long_paragraph_wraps_to_multiple_lines() {
    // ~600 characters is long enough to wrap on a Letter content area
    // (612 px wide @ 14 px font). We don't pin the exact line count; the
    // contract is "more than one line".
    let long = "lorem ipsum dolor sit amet ".repeat(40);
    let d = paragraph_doc(&long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let lines = plan.pages[0].blocks[0].lines.len();
    assert!(lines > 1, "long paragraph should wrap; got {lines} line(s)");
}

#[test]
fn line_widths_do_not_exceed_content_width() {
    let long = "lorem ipsum dolor sit amet ".repeat(40);
    let d = paragraph_doc(&long);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let cw = LETTER_AT_96DPI.content_width();
    for page in &plan.pages {
        for bb in &page.blocks {
            for line in &bb.lines {
                assert!(
                    line.width_px <= cw + 1.0,
                    "line width {} exceeds content width {}",
                    line.width_px,
                    cw,
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Viewport variations
// ─────────────────────────────────────────────────────────────────

#[test]
fn narrow_viewport_increases_wrap_count() {
    // 55-char text fits one line at 624 px content width but cannot fit
    // on a 152 px content width regardless of font choice — strictly
    // more lines on narrow.
    let text = "lorem ipsum dolor sit amet consectetur adipiscing elit";
    let d = paragraph_doc(text);
    let wide = LETTER_AT_96DPI;
    let narrow = Viewport {
        page_width_px: 200.0,
        page_height_px: 1056.0,
        margin_px: 24.0,
    };
    let n_wide = layout(&d, &wide).unwrap().pages[0].blocks[0].lines.len();
    let n_narrow = layout(&d, &narrow).unwrap().pages[0].blocks[0].lines.len();
    assert!(
        n_narrow > n_wide,
        "narrower viewport must wrap into strictly more lines: narrow={n_narrow} wide={n_wide}",
    );
}

#[test]
fn very_short_viewport_yields_at_least_one_page_per_block() {
    // A paragraph block is ~26 px tall (18 px line + 8 px space-after).
    // A 30 px content area cannot fit two consecutive paragraphs, so
    // every block ends up alone on its own page.
    let d = paragraphs(&["a", "b", "c", "d"]);
    let tiny = Viewport {
        page_width_px: 400.0,
        page_height_px: 30.0,
        margin_px: 0.0,
    };
    let plan = layout(&d, &tiny).unwrap();
    assert_eq!(
        plan.page_count(),
        d.block_count(),
        "tiny viewport should give exactly 1 page per block; pages={} blocks={}",
        plan.page_count(),
        d.block_count(),
    );
}

#[test]
fn content_width_helper_is_correct() {
    let v = LETTER_AT_96DPI;
    assert!((v.content_width() - 624.0).abs() < 1e-3);
    assert!((v.content_height() - 864.0).abs() < 1e-3);
}

// ─────────────────────────────────────────────────────────────────
// Determinism + pure-function semantics
// ─────────────────────────────────────────────────────────────────

#[test]
fn layout_is_deterministic_across_repeat_calls() {
    let d = paragraphs(&["one", "two", "three"]);
    let p1 = layout(&d, &LETTER_AT_96DPI).unwrap();
    let p2 = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(p1, p2);
}

#[test]
fn layout_is_deterministic_with_same_doc_built_independently() {
    let d1 = paragraphs(&["alpha", "beta"]);
    let d2 = paragraphs(&["alpha", "beta"]);
    let p1 = layout(&d1, &LETTER_AT_96DPI).unwrap();
    let p2 = layout(&d2, &LETTER_AT_96DPI).unwrap();
    assert_eq!(p1, p2);
}

#[test]
fn layout_does_not_mutate_doc_observable_state() {
    let d = paragraphs(&["one", "two"]);
    let before_text = d.text();
    let before_count = d.block_count();
    let _ = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(d.text(), before_text);
    assert_eq!(d.block_count(), before_count);
}

// ─────────────────────────────────────────────────────────────────
// Dirty rects (Phase 4.1: full content area per page)
// ─────────────────────────────────────────────────────────────────

#[test]
fn dirty_rects_present_for_every_page() {
    let d = paragraphs(&["x", "y", "z"]);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(
        plan.dirty_rects.len(),
        plan.pages.len(),
        "Phase 4.1 emits one dirty rect per page (full content area)",
    );
}

#[test]
fn dirty_rect_default_is_zero_sized() {
    let r = Rect::default();
    assert_eq!(r.x_px, 0.0);
    assert_eq!(r.width_px, 0.0);
    assert_eq!(r.height_px, 0.0);
}

#[test]
fn dirty_rect_covers_content_area() {
    let d = paragraph_doc("hello");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let r = plan.dirty_rects[0];
    let cw = LETTER_AT_96DPI.content_width();
    let ch = LETTER_AT_96DPI.content_height();
    assert!(
        (r.width_px - cw).abs() < 1e-3,
        "dirty width {} != content width {}",
        r.width_px,
        cw
    );
    assert!(
        (r.height_px - ch).abs() < 1e-3,
        "dirty height {} != content height {}",
        r.height_px,
        ch
    );
}

// ─────────────────────────────────────────────────────────────────
// RenderPlan accessors
// ─────────────────────────────────────────────────────────────────

#[test]
fn default_render_plan_is_empty() {
    let p = RenderPlan::default();
    assert_eq!(p.page_count(), 0);
    assert_eq!(p.block_box_count(), 0);
    assert!(p.dirty_rects.is_empty());
}

#[test]
fn page_count_matches_pages_len() {
    let d = paragraph_doc("only one");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.page_count(), plan.pages.len());
}

#[test]
fn block_box_count_sums_across_pages() {
    let texts: Vec<String> = (0..50).map(|i| format!("para {i}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let d = paragraphs(&refs);
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let by_page_sum: usize = plan.pages.iter().map(|p| p.blocks.len()).sum();
    assert_eq!(plan.block_box_count(), by_page_sum);
}

// ─────────────────────────────────────────────────────────────────
// Multibyte / LATAM / CJK
// ─────────────────────────────────────────────────────────────────

#[test]
fn latam_dead_key_text_lays_out_without_panic() {
    let d = paragraph_doc("Año cálido en España — niño, día, café");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), 1);
    assert!(plan.pages[0].blocks[0].height_px > 0.0);
}

#[test]
fn cjk_text_lays_out_without_panic() {
    let d = paragraph_doc("漢字 ひらがな カタカナ");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), 1);
}

#[test]
fn pua_marker_codepoints_pass_through_layout() {
    // U+E000 / U+E001 are the citation / footnote markers from
    // doc-model Phase D. They must not crash the layout engine.
    let d = paragraph_doc("text\u{E000}cite\u{E001}fn");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), 1);
}

// ─────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────

#[test]
fn empty_content_area_when_margin_consumes_page() {
    let d = paragraph_doc("hi");
    let bad = Viewport {
        page_width_px: 100.0,
        page_height_px: 100.0,
        margin_px: 60.0, // 2*60 = 120 > 100 → content area is negative
    };
    let err = layout(&d, &bad).expect_err("layout should reject collapsed viewport");
    assert!(
        matches!(err, Error::EmptyContentArea { .. }),
        "expected EmptyContentArea, got {err:?}",
    );
}

#[test]
fn zero_width_content_area_errs() {
    let d = paragraph_doc("hi");
    let bad = Viewport {
        page_width_px: 100.0,
        page_height_px: 1056.0,
        margin_px: 50.0, // 2*50 = 100 → width is exactly 0
    };
    assert!(matches!(
        layout(&d, &bad).unwrap_err(),
        Error::EmptyContentArea { .. },
    ));
}

#[test]
fn zero_height_content_area_errs() {
    let d = paragraph_doc("hi");
    let bad = Viewport {
        page_width_px: 1000.0,
        page_height_px: 50.0,
        margin_px: 25.0, // 2*25 = 50 → height is exactly 0
    };
    assert!(matches!(
        layout(&d, &bad).unwrap_err(),
        Error::EmptyContentArea { .. },
    ));
}

// ─────────────────────────────────────────────────────────────────
// Mixed-kind documents
// ─────────────────────────────────────────────────────────────────

#[test]
fn heading_then_paragraph_block_count_is_two() {
    let mut d = Doc::new();
    // Insert paragraph text "intro", then prepend a heading.
    d.insert(0, "intro");
    d.apply_edit_op(EditOp::InsertBlock {
        at: 0,
        block: Block {
            kind: BlockKind::Heading { level: 1 },
            text: "Title".into(),
        },
    })
    .unwrap();
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.block_box_count(), d.block_count());
    assert!(plan.block_box_count() >= 2);
}
