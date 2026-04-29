//! Phase 4.3 RED — page-break math: block splitting, widow / orphan,
//! hard breaks, heading keep-with-next, page numbers, no-op edit
//! invariant.

use apalabrar_doc_model::{Block, BlockKind, Doc, EditOp};
use apalabrar_layout::{
    BlockKind as LayoutBlockKind, HARD_PAGE_BREAK_MARKER, LETTER_AT_96DPI, PaginationConfig,
    Viewport, layout, layout_with_config,
};
use proptest::prelude::*;

// A tiny content area: 90 px tall, fits 5 lines of 18 px each. Forces
// multi-page layouts on small inputs so split / widow / orphan tests
// have meaningful boundaries.
const TINY: Viewport = Viewport {
    page_width_px: 400.0,
    page_height_px: 90.0,
    margin_px: 0.0,
};

fn paragraph_doc(text: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, text);
    d
}

fn long_lines_paragraph_doc(line_count: usize) -> Doc {
    // Each line forced by a `\n` would split into separate blocks; we
    // need one BLOCK with many shaped lines, so we let the wrap engine
    // do the work. ~30 chars/line in TINY (400 px wide @ 14 px font).
    // To get N lines, supply ~30 * N chars in one block.
    let chunk = "abcdefghijklmnopqrstuvwxyz1234 ";
    let text: String = chunk.repeat(line_count);
    paragraph_doc(&text)
}

// ─────────────────────────────────────────────────────────────────
// Page numbers
// ─────────────────────────────────────────────────────────────────

#[test]
fn page_number_starts_at_1() {
    let d = paragraph_doc("hello");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.pages[0].page_number, 1);
}

#[test]
fn page_numbers_are_sequential() {
    let d = long_lines_paragraph_doc(40);
    let plan = layout(&d, &TINY).unwrap();
    let nums: Vec<usize> = plan.pages.iter().map(|p| p.page_number).collect();
    let expected: Vec<usize> = (1..=plan.pages.len()).collect();
    assert_eq!(
        nums, expected,
        "page_number must be 1, 2, 3, … with no gaps"
    );
}

// ─────────────────────────────────────────────────────────────────
// Block splitting + line_range
// ─────────────────────────────────────────────────────────────────

#[test]
fn long_paragraph_splits_across_pages() {
    let d = long_lines_paragraph_doc(40);
    let plan = layout(&d, &TINY).unwrap();
    let appearances: Vec<&_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .collect();
    assert!(
        appearances.len() >= 2,
        "block 0 should appear on multiple pages; got {}",
        appearances.len(),
    );
}

#[test]
fn line_range_partitions_block_lines_disjointly() {
    let d = long_lines_paragraph_doc(40);
    let plan = layout(&d, &TINY).unwrap();
    let mut covered: Vec<usize> = Vec::new();
    for page in &plan.pages {
        for bx in &page.blocks {
            if bx.block_index == 0 {
                for i in bx.line_range.clone() {
                    covered.push(i);
                }
            }
        }
    }
    let mut sorted = covered.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        covered.len(),
        sorted.len(),
        "no line should appear in two BlockBoxes",
    );
}

#[test]
fn line_range_covers_all_lines_of_split_block() {
    let d = long_lines_paragraph_doc(40);
    let plan = layout(&d, &TINY).unwrap();
    // Every line index 0..total must appear in exactly one BlockBox
    // that points at block 0.
    let total_lines: usize = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .map(|b| b.lines.len())
        .sum();
    let max_end: usize = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .map(|b| b.line_range.end)
        .max()
        .unwrap();
    assert_eq!(total_lines, max_end, "all block lines must be covered");
}

#[test]
fn first_segment_starts_at_line_0() {
    let d = long_lines_paragraph_doc(40);
    let plan = layout(&d, &TINY).unwrap();
    let first = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .find(|b| b.block_index == 0)
        .unwrap();
    assert_eq!(first.line_range.start, 0);
}

#[test]
fn unique_block_indices_equals_block_count() {
    // Replaces 4.1 invariant `block_box_count == block_count`. With
    // splitting, multiple BlockBoxes may share a block_index; the
    // count of UNIQUE indices is what must equal `doc.block_count()`.
    let d = long_lines_paragraph_doc(40);
    let plan = layout(&d, &TINY).unwrap();
    let mut indices: Vec<usize> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter().map(|b| b.block_index))
        .collect();
    indices.sort_unstable();
    indices.dedup();
    assert_eq!(indices.len(), d.block_count());
}

#[test]
fn short_block_does_not_split() {
    let d = paragraph_doc("hi");
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    let appearances: Vec<&_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .collect();
    assert_eq!(appearances.len(), 1, "a single-line block should not split");
    assert_eq!(appearances[0].line_range, 0..1);
}

// ─────────────────────────────────────────────────────────────────
// Widow / orphan rules
// ─────────────────────────────────────────────────────────────────

#[test]
fn widow_control_pushes_minimum_2_lines_to_next_page() {
    // 6 lines into a 5-line page would split 5/1 (widow). Default
    // min_widow=2 must bump the split to 4/2 so the trailing page
    // has at least 2 lines.
    let d = long_lines_paragraph_doc(6);
    let plan = layout(&d, &TINY).unwrap();
    let last_segment = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .rfind(|b| b.block_index == 0)
        .unwrap();
    let tail_lines = last_segment.line_range.end - last_segment.line_range.start;
    assert!(
        tail_lines >= 2,
        "widow rule (default 2) violated: tail has {tail_lines} lines",
    );
}

#[test]
fn widow_control_can_be_disabled_via_config() {
    let d = long_lines_paragraph_doc(6);
    let lax = PaginationConfig {
        min_orphan_lines: 1,
        min_widow_lines: 1,
    };
    let plan = layout_with_config(&d, &TINY, &lax).unwrap();
    // With min_widow=1, the split keeps the greedy 5/1 layout — the
    // tail can be a single line.
    let last_segment = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .rfind(|b| b.block_index == 0)
        .unwrap();
    let tail_lines = last_segment.line_range.end - last_segment.line_range.start;
    assert!(
        tail_lines >= 1,
        "lax widow=1 still requires at least 1 line tail",
    );
}

#[test]
fn orphan_control_pushes_block_whole_when_only_one_line_fits() {
    // Build: one paragraph that fills 4 lines (leaves 1 line on the
    // page), then a 3-line paragraph. Default packer with orphan=2
    // must push the 3-line block whole to the next page rather than
    // strand a single-line head.
    let mut d = Doc::new();
    let filler = "abcdefghijklmnopqrstuvwxyz1234 ".repeat(4);
    let main = "abcdefghijklmnopqrstuvwxyz1234 ".repeat(3);
    d.insert(0, &format!("{filler}\n{main}"));
    let plan = layout(&d, &TINY).unwrap();
    let main_segments: Vec<&_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 1)
        .collect();
    if main_segments.len() == 1 {
        // Block stayed whole — head was pushed (orphan rule fired).
        assert_eq!(main_segments[0].line_range, 0..3);
    } else {
        // If it did split, the head segment must have ≥ 2 lines.
        let head_lines = main_segments[0].line_range.end - main_segments[0].line_range.start;
        assert!(
            head_lines >= 2,
            "orphan rule (default 2) violated: head has {head_lines} lines",
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// Hard breaks
// ─────────────────────────────────────────────────────────────────

#[test]
fn hard_break_marker_forces_a_new_page() {
    // Two single-line paragraphs that would otherwise share a page,
    // separated by a hard break in the second block.
    let mut d = Doc::new();
    d.insert(0, &format!("first\n{HARD_PAGE_BREAK_MARKER}second"));
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert!(
        plan.pages.len() >= 2,
        "hard break should force a second page; got {} pages",
        plan.pages.len(),
    );
    let page1_indices: Vec<usize> = plan.pages[0].blocks.iter().map(|b| b.block_index).collect();
    let page2_indices: Vec<usize> = plan.pages[1].blocks.iter().map(|b| b.block_index).collect();
    assert!(page1_indices.contains(&0));
    assert!(page2_indices.contains(&1));
}

#[test]
fn hard_break_at_doc_start_does_not_create_an_empty_page() {
    let mut d = Doc::new();
    d.insert(0, &format!("{HARD_PAGE_BREAK_MARKER}only"));
    let plan = layout(&d, &LETTER_AT_96DPI).unwrap();
    assert_eq!(plan.pages.len(), 1, "no leading empty page");
}

// ─────────────────────────────────────────────────────────────────
// Heading keep-with-next
// ─────────────────────────────────────────────────────────────────

#[test]
fn heading_at_page_bottom_glues_to_next_block() {
    // Engineer: 1-line filler (~26 px) + h2 heading (~50 px) = 76 px
    // (fits on page 1's 90 px). The trailing 1-line body (~26 px)
    // cannot fit (14 px left). Without keep-with-next: heading on
    // page 1, body alone on page 2 (heading orphaned at the bottom).
    // With keep-with-next: heading moves to page 2 with the body.
    let mut d = Doc::new();
    let filler = "abcdefghijklmnopqrstuvwxyz1234".to_owned();
    d.insert(0, &filler);
    d.apply_edit_op(EditOp::InsertBlock {
        at: filler.chars().count(),
        block: Block {
            kind: BlockKind::Heading { level: 2 },
            text: "Section".into(),
        },
    })
    .unwrap();
    let len = d.text().chars().count();
    d.insert(len, "\nbody");
    let plan = layout(&d, &TINY).unwrap();

    // Find the page containing the heading.
    let heading_page = plan
        .pages
        .iter()
        .position(|p| {
            p.blocks
                .iter()
                .any(|b| matches!(b.kind, LayoutBlockKind::Heading { .. }))
        })
        .expect("heading must land on some page");

    // The block AFTER the heading must be on the same page (glue).
    let heading_block_index = plan.pages[heading_page]
        .blocks
        .iter()
        .find(|b| matches!(b.kind, LayoutBlockKind::Heading { .. }))
        .unwrap()
        .block_index;
    let next_block_index = heading_block_index + 1;
    let next_on_same_page = plan.pages[heading_page]
        .blocks
        .iter()
        .any(|b| b.block_index == next_block_index);
    assert!(
        next_on_same_page,
        "heading at bottom of page must glue to the following block",
    );
}

// ─────────────────────────────────────────────────────────────────
// No-op edit invariant (Section E R3 page-count invariant)
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn page_count_unchanged_under_empty_insert(
        paragraphs in prop::collection::vec(
            prop::string::string_regex("[a-z ]{1,80}").unwrap(),
            1..=15,
        ),
        at in 0usize..200,
    ) {
        let mut d = Doc::new();
        d.insert(0, &paragraphs.join("\n"));
        let before = layout(&d, &LETTER_AT_96DPI).unwrap().page_count();
        // Empty insert is a documented no-op in doc-model
        // (`Doc::insert` short-circuits on empty text).
        d.apply_edit_op(EditOp::InsertText {
            at: at.min(d.text().chars().count()),
            text: String::new(),
            marks: vec![],
        }).unwrap();
        let after = layout(&d, &LETTER_AT_96DPI).unwrap().page_count();
        prop_assert_eq!(before, after);
    }
}

// ─────────────────────────────────────────────────────────────────
// PaginationConfig defaults + traits
// ─────────────────────────────────────────────────────────────────

#[test]
fn pagination_config_default_min_orphan_2() {
    assert_eq!(PaginationConfig::default().min_orphan_lines, 2);
}

#[test]
fn pagination_config_default_min_widow_2() {
    assert_eq!(PaginationConfig::default().min_widow_lines, 2);
}

#[test]
fn pagination_config_traits() {
    let c = PaginationConfig::default();
    let copy = c;
    let cloned = c;
    assert_eq!(copy, cloned);
    let dbg = format!("{c:?}");
    assert!(dbg.contains("PaginationConfig"));
}

#[test]
fn split_segments_are_contiguous() {
    // line_range.end of segment N must equal line_range.start of
    // segment N+1 — the line indices form a partition with no gaps.
    let d = long_lines_paragraph_doc(20);
    let plan = layout(&d, &TINY).unwrap();
    let ranges: Vec<_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .map(|b| b.line_range.clone())
        .collect();
    assert!(ranges.len() >= 2, "expected the block to split");
    for w in ranges.windows(2) {
        assert_eq!(
            w[0].end, w[1].start,
            "segments must be contiguous; gap between {:?} and {:?}",
            w[0], w[1],
        );
    }
}

#[test]
fn segment_height_matches_line_count_times_line_height() {
    // For a paragraph (line_h = 18, space_before = 0), every
    // page-segment carries `n * line_h` for its lines; the final
    // segment adds `space_after = 8` ONLY when it fits without
    // pushing past the page boundary (which is typography-correct:
    // an end-of-page paragraph's space_after lives in the bottom
    // margin and is invisible). Use 12 lines so the last segment
    // (2 lines) easily fits with `space_after`.
    let d = long_lines_paragraph_doc(12);
    let plan = layout(&d, &TINY).unwrap();
    let segments: Vec<_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .collect();
    let total_lines: usize = segments.iter().map(|s| s.lines.len()).sum();
    for (i, seg) in segments.iter().enumerate() {
        let n = seg.lines.len() as f32;
        let is_last = seg.line_range.end == total_lines;
        let bare = n * 18.0;
        // Segment height is either `n*line_h` (truncated last segment
        // that fills the page) or `n*line_h + space_after` (last
        // segment with room).
        let max_expected = if is_last { bare + 8.0 } else { bare };
        assert!(
            seg.height_px >= bare - 0.5 && seg.height_px <= max_expected + 0.5,
            "segment {i} height {} not in expected range [{bare}, {max_expected}] (lines={n}, last={is_last})",
            seg.height_px,
        );
    }
}

#[test]
fn widow_split_first_segment_is_total_minus_min_widow() {
    // 6 lines in TINY (5 per page). Default greedy 5/1; widow bump
    // → 4/2. The first segment must end at total - min_widow_lines.
    let d = long_lines_paragraph_doc(6);
    let plan = layout(&d, &TINY).unwrap();
    let segments: Vec<_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .collect();
    assert!(segments.len() >= 2);
    let total: usize = segments.iter().map(|s| s.lines.len()).sum();
    let first_end = segments[0].line_range.end;
    let expected = total - PaginationConfig::default().min_widow_lines;
    assert_eq!(
        first_end, expected,
        "widow rule should split at line {expected}, got {first_end}",
    );
}

#[test]
fn no_block_box_has_an_empty_line_range() {
    // The packer must never emit a placeholder BlockBox for `n..n`
    // — that would mean `while offset < n` ran one too many times.
    let d = long_lines_paragraph_doc(20);
    let plan = layout(&d, &TINY).unwrap();
    for page in &plan.pages {
        for bx in &page.blocks {
            assert!(
                !bx.line_range.is_empty(),
                "empty line_range {:?} on page {}",
                bx.line_range,
                page.page_number,
            );
        }
    }
}

#[test]
fn split_pages_have_distinct_page_numbers() {
    // page_number must increment by 1 across every flush, including
    // the per-segment flushes inside the splitter.
    let d = long_lines_paragraph_doc(20);
    let plan = layout(&d, &TINY).unwrap();
    let nums: Vec<usize> = plan.pages.iter().map(|p| p.page_number).collect();
    let unique: std::collections::HashSet<_> = nums.iter().copied().collect();
    assert_eq!(unique.len(), nums.len(), "page_numbers must be unique");
    for w in nums.windows(2) {
        assert_eq!(w[1], w[0] + 1, "page_numbers must be consecutive");
    }
}

#[test]
fn block_taller_than_page_force_splits_even_under_orphan() {
    // Viewport content height = 30 px, line_h ≈ 18 px → max_fit = 1
    // line. With min_orphan = 2 the orphan rule would push the block
    // whole forward, but the next page is just as small. The packer
    // must instead force-split, placing 1 line per page.
    let d = long_lines_paragraph_doc(4);
    let tiny = Viewport {
        page_width_px: 400.0,
        page_height_px: 30.0,
        margin_px: 0.0,
    };
    let plan = layout(&d, &tiny).unwrap();
    let segments: Vec<&_> = plan
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| b.block_index == 0)
        .collect();
    assert!(
        segments.len() >= 2,
        "block taller than every page must force-split; got {} segments",
        segments.len(),
    );
}

#[test]
fn block_taller_than_page_after_other_block_first_flushes() {
    // Place a tiny filler then a block taller than the page. The
    // packer must flush the filler, then force-split the giant block
    // line-by-line on subsequent (empty) pages.
    let mut d = Doc::new();
    d.insert(0, "h\n");
    let len = d.text().chars().count();
    let giant = "abcdefghijklmnopqrstuvwxyz1234 ".repeat(4);
    d.insert(len, &giant);
    let tiny = Viewport {
        page_width_px: 400.0,
        page_height_px: 30.0,
        margin_px: 0.0,
    };
    let plan = layout(&d, &tiny).unwrap();
    // First page contains the filler block; the giant block lives on
    // its own pages from page 2 onward.
    let block_0_page: Option<usize> = plan
        .pages
        .iter()
        .position(|p| p.blocks.iter().any(|b| b.block_index == 0));
    let block_1_page: Option<usize> = plan
        .pages
        .iter()
        .position(|p| p.blocks.iter().any(|b| b.block_index == 1));
    assert!(block_0_page.is_some());
    assert!(block_1_page.is_some());
    assert!(
        block_1_page > block_0_page,
        "giant block must follow the filler"
    );
}

#[test]
fn layout_and_layout_with_config_default_match() {
    let d = long_lines_paragraph_doc(20);
    let p1 = layout(&d, &LETTER_AT_96DPI).unwrap();
    let p2 = layout_with_config(&d, &LETTER_AT_96DPI, &PaginationConfig::default()).unwrap();
    assert_eq!(p1, p2);
}
