//! Phase 4.5 RED — `resolve_selection` tests.
//!
//! Maps a doc-level codepoint range to per-line bounding rects in
//! page-local coordinates. Used by the editor surface to paint
//! find-highlights and the selection overlay without re-running
//! `layout()`.

use apalabrar_doc_model::Doc;
use apalabrar_layout::{LETTER_AT_96DPI, layout, resolve_selection};

fn paragraph_doc(text: &str) -> Doc {
    let mut d = Doc::new();
    d.insert(0, text);
    d
}

#[test]
fn resolve_selection_empty_range_returns_no_rects() {
    let doc = paragraph_doc("hello world");
    let plan = layout(&doc, &LETTER_AT_96DPI).unwrap();
    // Range [3..3) is empty (start == end); spec says vec![].
    assert_eq!(resolve_selection(&plan, &doc.text(), 3..3), Vec::new());
}

#[test]
fn resolve_selection_single_line_range_returns_one_rect() {
    let doc = paragraph_doc("hello world");
    let plan = layout(&doc, &LETTER_AT_96DPI).unwrap();
    // "world" occupies codepoints [6..11). On a Letter page at 96 DPI
    // with a single short paragraph, this is one shaped line → one Rect.
    let rects = resolve_selection(&plan, &doc.text(), 6..11);
    assert_eq!(
        rects.len(),
        1,
        "single-line range must produce exactly one rect, got {:?}",
        rects
    );
    // The rect must sit on the first page, with positive width covering
    // the glyphs of "world" (5 chars at default font size).
    assert!(
        rects[0].width_px > 0.0,
        "rect width must be positive: {:?}",
        rects[0]
    );
    assert!(
        rects[0].height_px > 0.0,
        "rect height must be positive: {:?}",
        rects[0]
    );
}

#[test]
fn resolve_selection_multi_line_range_returns_one_rect_per_line() {
    // Two paragraphs separated by \n — a range that spans the boundary
    // covers two distinct lines, so two rects on different y positions.
    let mut doc = Doc::new();
    doc.insert(0, "hello\nworld");
    let plan = layout(&doc, &LETTER_AT_96DPI).unwrap();
    // Range [3..9) covers "lo" on line 1 (across the \n at codepoint 5)
    // and "wor" on line 2.
    let rects = resolve_selection(&plan, &doc.text(), 3..9);
    assert_eq!(rects.len(), 2, "two lines → two rects, got {:?}", rects);
    // Stronger: the two rects must be on different lines, so their
    // y_px must differ. (A buggy impl returning two copies of the same
    // line would pass the len check but fail this.)
    assert_ne!(
        rects[0].y_px, rects[1].y_px,
        "two-line rects must have distinct y_px: {:?}",
        rects,
    );
}

#[test]
fn resolve_selection_clips_range_past_end_of_doc() {
    let doc = paragraph_doc("hello");
    let plan = layout(&doc, &LETTER_AT_96DPI).unwrap();
    // Asking for [3..999) — must clip to [3..5) and return one rect for
    // "lo" rather than panic / overflow.
    let rects = resolve_selection(&plan, &doc.text(), 3..999);
    assert_eq!(rects.len(), 1);
    assert!(rects[0].width_px > 0.0);
}
