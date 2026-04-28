//! Behavioural tests for the layout engine. RED until GREEN flips the
//! `todo!()` bodies in `lib.rs`. Coverage targets:
//!
//! - Construction + viewport math
//! - Empty document → 0 pages
//! - Single block → 1 page, 1 box
//! - Many short blocks → multiple pages (pagination triggers)
//! - Block kinds (Paragraph, Heading, ListItem) preserved + visually distinct
//! - Heading taller than paragraph for the same text
//! - List indent reflected in `origin_x_px`
//! - LATAM accents survive shaping (no panic, text round-trips through the
//!   shape path)
//! - Determinism: identical input twice → identical output
//! - Cache populated after layout
//! - `relayout_after_change` re-shapes only the changed block + matches a
//!   fresh `layout(modified)` result
//! - `relayout_after_change` out-of-bounds returns `InvalidBlockIndex`
//! - `relayout_after_change` on a length-changed doc transparently rebuilds

use apalabrar_layout::{Block, BlockKind, Document, Engine, Error, LETTER_AT_96DPI, Viewport};

fn engine() -> Engine {
    Engine::new(LETTER_AT_96DPI).expect("bundled font loads")
}

fn paragraph(text: &str) -> Block {
    Block::Paragraph {
        text: text.to_owned(),
    }
}

fn heading(level: u8, text: &str) -> Block {
    Block::Heading {
        level,
        text: text.to_owned(),
    }
}

fn list_item(indent: u8, text: &str) -> Block {
    Block::ListItem {
        indent,
        text: text.to_owned(),
    }
}

// ---------- Construction + viewport ----------

#[test]
fn engine_constructs_with_letter_viewport() {
    let engine = engine();
    assert_eq!(engine.cached_block_count(), 0);
    assert_eq!(engine.cached_doc_len(), 0);
}

#[test]
fn viewport_content_area_is_page_minus_two_margins() {
    let v = LETTER_AT_96DPI;
    assert_eq!(v.content_width(), v.page_width_px - 2.0 * v.margin_px);
    assert_eq!(v.content_height(), v.page_height_px - 2.0 * v.margin_px);
}

#[test]
fn viewport_with_zero_margin_uses_full_page() {
    let v = Viewport {
        page_width_px: 100.0,
        page_height_px: 200.0,
        margin_px: 0.0,
    };
    assert_eq!(v.content_width(), 100.0);
    assert_eq!(v.content_height(), 200.0);
}

// ---------- Empty document ----------

#[test]
fn empty_document_lays_out_to_zero_pages() {
    let mut e = engine();
    let result = e.layout(&Document::default());
    assert_eq!(
        result.page_count(),
        0,
        "empty doc should produce no pages; renderer can decide to show a blank canvas",
    );
    assert_eq!(result.block_box_count(), 0);
    assert_eq!(e.cached_block_count(), 0);
    assert_eq!(e.cached_doc_len(), 0);
}

// ---------- Single block ----------

#[test]
fn single_short_paragraph_fits_on_one_page_with_one_box() {
    let mut e = engine();
    let doc = Document::new(vec![paragraph("Hello, world.")]);
    let result = e.layout(&doc);
    assert_eq!(result.page_count(), 1);
    assert_eq!(result.block_box_count(), 1);
    let bx = &result.pages[0].blocks[0];
    assert_eq!(bx.block_index, 0);
    assert_eq!(bx.kind, BlockKind::Paragraph);
    assert!(
        !bx.lines.is_empty(),
        "paragraph must produce at least one line"
    );
}

#[test]
fn single_paragraph_block_box_count_equals_doc_len() {
    let mut e = engine();
    let doc = Document::new(vec![paragraph("hi")]);
    let result = e.layout(&doc);
    assert_eq!(result.block_box_count(), doc.len());
}

#[test]
fn cached_block_count_after_layout_equals_doc_len() {
    let mut e = engine();
    let doc = Document::new(vec![
        paragraph("first"),
        heading(1, "Second"),
        list_item(0, "third"),
    ]);
    let _ = e.layout(&doc);
    assert_eq!(e.cached_block_count(), 3);
    assert_eq!(e.cached_doc_len(), 3);
}

// ---------- Pagination ----------

#[test]
fn many_short_paragraphs_overflow_to_multiple_pages() {
    let mut e = engine();
    // 200 short paragraphs at 14 px line height + spacing easily exceeds one
    // 864-px content area (Letter at 96 DPI minus 2×96-px margins).
    let blocks: Vec<Block> = (0..200)
        .map(|i| paragraph(&format!("Block number {i}.")))
        .collect();
    let doc = Document::new(blocks);
    let result = e.layout(&doc);
    assert!(
        result.page_count() > 1,
        "expected multi-page result; got {} pages",
        result.page_count(),
    );
    assert_eq!(
        result.block_box_count(),
        doc.len(),
        "every block in the doc must end up in exactly one page",
    );
}

#[test]
fn pagination_assigns_each_block_index_exactly_once() {
    let mut e = engine();
    let doc = Document::new(
        (0..120)
            .map(|i| paragraph(&format!("paragraph {i}")))
            .collect(),
    );
    let result = e.layout(&doc);
    let mut seen: Vec<usize> = result
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter().map(|b| b.block_index))
        .collect();
    seen.sort_unstable();
    let expected: Vec<usize> = (0..doc.len()).collect();
    assert_eq!(
        seen, expected,
        "block_index 0..len must each appear exactly once across all pages",
    );
}

// ---------- Block kinds ----------

#[test]
fn block_kind_paragraph_round_trips() {
    let mut e = engine();
    let result = e.layout(&Document::new(vec![paragraph("p")]));
    assert_eq!(result.pages[0].blocks[0].kind, BlockKind::Paragraph);
}

#[test]
fn block_kind_heading_round_trips_with_level() {
    let mut e = engine();
    let result = e.layout(&Document::new(vec![heading(2, "h2")]));
    assert_eq!(
        result.pages[0].blocks[0].kind,
        BlockKind::Heading { level: 2 },
    );
}

#[test]
fn block_kind_list_item_round_trips_with_indent() {
    let mut e = engine();
    let result = e.layout(&Document::new(vec![list_item(3, "item")]));
    assert_eq!(
        result.pages[0].blocks[0].kind,
        BlockKind::ListItem { indent: 3 },
    );
}

#[test]
fn heading_height_is_greater_than_paragraph_height_for_same_text() {
    let mut e = engine();
    let p_result = e.layout(&Document::new(vec![paragraph("Same text content.")]));
    let h_result = e.layout(&Document::new(vec![heading(1, "Same text content.")]));
    let p_box = &p_result.pages[0].blocks[0];
    let h_box = &h_result.pages[0].blocks[0];
    assert!(
        h_box.height_px > p_box.height_px,
        "h1 box ({} px) must be taller than paragraph box ({} px) for the same text",
        h_box.height_px,
        p_box.height_px,
    );
}

#[test]
fn list_item_indent_increases_origin_x() {
    let mut e = engine();
    let no_indent = e.layout(&Document::new(vec![list_item(0, "flat list item")]));
    let deep = e.layout(&Document::new(vec![list_item(3, "deep list item")]));
    let flat_x = no_indent.pages[0].blocks[0].origin_x_px;
    let deep_x = deep.pages[0].blocks[0].origin_x_px;
    assert!(
        deep_x > flat_x,
        "indent=3 origin_x ({deep_x}) must exceed indent=0 origin_x ({flat_x})",
    );
}

// ---------- LATAM accents ----------

#[test]
fn latam_accents_survive_layout() {
    let mut e = engine();
    let text = "Ñoño año mañana día sí. Á É Í Ó Ú Ü ç ¿ ¡";
    let doc = Document::new(vec![paragraph(text)]);
    let result = e.layout(&doc);
    assert_eq!(result.block_box_count(), 1);
    let bx = &result.pages[0].blocks[0];
    assert!(
        !bx.lines.is_empty(),
        "shaped LATAM text must produce ≥1 line"
    );
}

// ---------- Determinism ----------

#[test]
fn layout_is_deterministic_for_same_input() {
    let mut e = engine();
    let doc = Document::new(vec![
        paragraph("alpha"),
        heading(1, "Beta"),
        list_item(1, "gamma"),
    ]);
    let a = e.layout(&doc);
    let b = e.layout(&doc);
    assert_eq!(a, b, "same doc shaped twice must produce equal results");
}

// ---------- Incremental relayout ----------

#[test]
fn relayout_after_change_oob_returns_invalid_block_index() {
    let mut e = engine();
    let doc = Document::new(vec![paragraph("only")]);
    let _ = e.layout(&doc);
    let result = e.relayout_after_change(&doc, 5);
    assert!(
        matches!(result, Err(Error::InvalidBlockIndex { index, len }) if index == 5 && len == 1)
    );
}

#[test]
fn relayout_after_change_in_place_matches_fresh_layout() {
    // Build a doc, lay it out, mutate one block, relayout incrementally,
    // and assert the result is bit-identical to a fresh full layout of the
    // mutated doc. Catches any cache-staleness bugs in the incremental path.
    let initial_blocks: Vec<Block> = (0..30)
        .map(|i| paragraph(&format!("body paragraph number {i}")))
        .collect();
    let mut doc = Document::new(initial_blocks);

    let mut incremental_engine = engine();
    let _ = incremental_engine.layout(&doc);

    // Mutate the middle block.
    if let Block::Paragraph { text } = &mut doc.blocks[15] {
        text.push_str(" — appended text");
    }
    let incremental = incremental_engine
        .relayout_after_change(&doc, 15)
        .expect("relayout should succeed for valid index");

    let mut fresh_engine = engine();
    let fresh = fresh_engine.layout(&doc);

    assert_eq!(
        incremental, fresh,
        "incremental relayout must match a fresh full layout of the mutated doc",
    );
}

#[test]
fn relayout_after_change_with_no_mutation_matches_fresh_layout() {
    // No-op edit (caller says block 5 changed but text is unchanged): the
    // result must still equal a fresh layout — guards against the incremental
    // path divergent due to cached metadata staleness.
    let blocks: Vec<Block> = (0..10).map(|i| paragraph(&format!("block {i}"))).collect();
    let doc = Document::new(blocks);

    let mut e = engine();
    let _ = e.layout(&doc);
    let incremental = e
        .relayout_after_change(&doc, 5)
        .expect("relayout valid index");

    let mut fresh_engine = engine();
    let fresh = fresh_engine.layout(&doc);

    assert_eq!(incremental, fresh);
}

#[test]
fn relayout_after_length_change_falls_back_to_full_layout() {
    // Append a block to the document then relayout with the index of the new
    // block. The cache is for the old length; the engine must transparently
    // rebuild instead of returning a stale or partially-populated layout.
    let mut e = engine();
    let mut doc = Document::new(vec![paragraph("original")]);
    let _ = e.layout(&doc);

    doc.blocks.push(paragraph("appended"));
    let result = e
        .relayout_after_change(&doc, 1)
        .expect("relayout should fall back rather than fail on a length change");
    assert_eq!(result.block_box_count(), 2);
    assert_eq!(e.cached_doc_len(), 2);
}

// ---------- Numeric sanity ----------

#[test]
fn block_box_height_equals_sum_of_line_heights_at_least() {
    // The block box's height must be at least as large as the sum of its
    // line heights — kind-specific spacing can add to it but never make it
    // smaller. Catches any regression where height_px is computed from a
    // disjoint path that doesn't reflect the lines we actually emit.
    let mut e = engine();
    let result = e.layout(&Document::new(vec![paragraph(
        "A reasonably long paragraph that should wrap into at least two lines \
         given the Letter-at-96-DPI viewport with one-inch margins and the \
         default body metrics.",
    )]));
    let bx = &result.pages[0].blocks[0];
    let line_sum: f32 = bx.lines.iter().map(|l| l.height_px).sum();
    assert!(
        bx.height_px >= line_sum - 0.001,
        "block height {} must be ≥ sum of line heights {}",
        bx.height_px,
        line_sum,
    );
}

// ---------- Document helpers ----------

#[test]
fn document_default_is_empty() {
    let doc = Document::default();
    assert!(doc.is_empty(), "default doc must report is_empty() == true");
    assert_eq!(doc.len(), 0);
}

#[test]
fn document_with_one_block_is_not_empty() {
    let doc = Document::new(vec![paragraph("x")]);
    assert!(
        !doc.is_empty(),
        "1-block doc must report is_empty() == false",
    );
    assert_eq!(doc.len(), 1);
}

// ---------- Wrap-width sensitivity to indent ----------

#[test]
fn list_item_with_higher_indent_wraps_into_more_lines() {
    // Same long text, different indent. The wrap width is content_width minus
    // left_inset, so deeper indent ⇒ narrower line ⇒ more line breaks. Test
    // catches a sign-flip on the wrap-width subtraction in shape_block.
    let mut e = engine();
    let body = "a".to_owned() + &" b".repeat(120); // ~240 chars, no long word so it wraps freely.
    let flat = e.layout(&Document::new(vec![list_item(0, &body)]));
    let deep = e.layout(&Document::new(vec![list_item(4, &body)]));
    let flat_lines = flat.pages[0].blocks[0].lines.len();
    let deep_lines = deep.pages[0].blocks[0].lines.len();
    assert!(
        deep_lines > flat_lines,
        "deep indent ({deep_lines} lines) should wrap into more lines than flat indent ({flat_lines} lines)",
    );
}

// ---------- Block-box width math ----------

#[test]
fn block_box_width_equals_content_width_minus_inset() {
    // Width is content_width - left_inset. Catches the mutation that flips
    // the subtraction sign or replaces it with division.
    let mut e = engine();
    let result = e.layout(&Document::new(vec![
        paragraph("flat"),
        list_item(0, "list zero"),
        list_item(3, "list three"),
    ]));
    let cw = LETTER_AT_96DPI.content_width();
    let p_box = &result.pages[0].blocks[0];
    let l0_box = &result.pages[0].blocks[1];
    let l3_box = &result.pages[0].blocks[2];

    // Paragraph has zero inset, so width = content_width.
    assert!(
        (p_box.width_px - cw).abs() < 0.01,
        "paragraph width {} should equal content_width {cw}",
        p_box.width_px,
    );

    // ListItem inset = 24 + 16 * indent (24 px base reserves space for the
    // bullet glyph; 16 px per nesting level).
    let expected_l0 = cw - (24.0 + 16.0 * 0.0);
    let expected_l3 = cw - (24.0 + 16.0 * 3.0);
    assert!(
        (l0_box.width_px - expected_l0).abs() < 0.01,
        "list-indent-0 width {} should equal content_width - bullet-inset = {expected_l0}",
        l0_box.width_px,
    );
    assert!(
        (l3_box.width_px - expected_l3).abs() < 0.01,
        "list-indent-3 width {} should equal content_width - inset = {expected_l3}",
        l3_box.width_px,
    );

    // Width must monotonically decrease with indent.
    assert!(l3_box.width_px < l0_box.width_px);
}

#[test]
fn long_paragraph_wraps_to_multiple_lines() {
    // 1000-char paragraph at body metrics inside a Letter-minus-2"-margin
    // content area must wrap into many lines; the test guards against a
    // regression that lets text overflow horizontally instead of wrapping.
    let mut e = engine();
    let long = "lorem ipsum dolor sit amet ".repeat(40);
    let result = e.layout(&Document::new(vec![paragraph(&long)]));
    let bx = &result.pages[0].blocks[0];
    assert!(
        bx.lines.len() > 5,
        "expected long paragraph to wrap into > 5 lines; got {}",
        bx.lines.len(),
    );
}
