//! CommonMark spec subset + GFM compliance tests for `read_md`.
//!
//! Curated representative inputs across the headline CommonMark
//! sections (headings, paragraphs, lists, code, block quotes,
//! thematic breaks, links, emphasis, hard breaks, HTML inline)
//! plus GFM extras (tables, footnotes, task lists, strikethrough).
//! Each test asserts a specific observable output of the parser:
//! paragraph_count, block_kind(i), or paragraph_text(i).
//!
//! These are RED tests on `read_md` until Step 4 wires the parser.

use apalabrar_format_md::{BlockKind, read_md};

// ---------- Headings (CommonMark §4.2-4.3) ----------

#[test]
fn parses_atx_h1() {
    let doc = read_md("# Title").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(1)));
    assert_eq!(doc.paragraph_text(0), Some("Title"));
}

#[test]
fn parses_atx_h2_through_h6() {
    let src = "## Two\n\n### Three\n\n#### Four\n\n##### Five\n\n###### Six";
    let doc = read_md(src).expect("parse");
    assert_eq!(doc.paragraph_count(), 5);
    for (i, lvl) in [(0, 2), (1, 3), (2, 4), (3, 5), (4, 6)] {
        assert_eq!(
            doc.block_kind(i),
            Some(BlockKind::Heading(lvl)),
            "block {i} should be H{lvl}"
        );
    }
}

#[test]
fn parses_setext_h1_with_underline() {
    let doc = read_md("Title\n=====\n").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(1)));
    assert_eq!(doc.paragraph_text(0), Some("Title"));
}

#[test]
fn parses_setext_h2_with_dashes() {
    let doc = read_md("Sub\n---\n").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(2)));
}

#[test]
fn heading_with_trailing_hashes_strips_them() {
    let doc = read_md("## Two ##").expect("parse");
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(2)));
    assert_eq!(doc.paragraph_text(0), Some("Two"));
}

#[test]
fn seven_hashes_is_paragraph_not_heading() {
    // CommonMark §4.2: 1-6 hashes are headings; 7+ is a paragraph.
    let doc = read_md("####### NotAHeading").expect("parse");
    assert_eq!(doc.block_kind(0), Some(BlockKind::Paragraph));
}

// ---------- Paragraphs (CommonMark §4.8) ----------

#[test]
fn parses_single_paragraph() {
    let doc = read_md("Hello world.").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Paragraph));
    assert_eq!(doc.paragraph_text(0), Some("Hello world."));
}

#[test]
fn parses_two_paragraphs_separated_by_blank_line() {
    let doc = read_md("First.\n\nSecond.").expect("parse");
    assert_eq!(doc.paragraph_count(), 2);
    assert_eq!(doc.paragraph_text(0), Some("First."));
    assert_eq!(doc.paragraph_text(1), Some("Second."));
}

#[test]
fn paragraph_text_concatenates_inline_runs() {
    let doc = read_md("Plain *italic* and **bold** text.").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    let text = doc.paragraph_text(0).unwrap();
    assert!(text.contains("Plain"));
    assert!(text.contains("italic"));
    assert!(text.contains("bold"));
    assert!(text.contains("text"));
}

#[test]
fn paragraph_with_inline_code() {
    let doc = read_md("Use `printf` for output.").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    let text = doc.paragraph_text(0).unwrap();
    assert!(text.contains("printf"));
}

#[test]
fn paragraph_with_link_includes_link_text() {
    let doc = read_md("See [docs](https://example.com) for details.").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    let text = doc.paragraph_text(0).unwrap();
    assert!(text.contains("docs"));
    assert!(text.contains("details"));
}

#[test]
fn paragraph_with_image_includes_alt_text() {
    let doc = read_md("![alt-text](image.png)").expect("parse");
    let text = doc.paragraph_text(0).unwrap();
    assert!(text.contains("alt-text"));
}

// ---------- Lists (CommonMark §5.2-5.3) ----------

#[test]
fn parses_unordered_list_as_one_block() {
    let doc = read_md("- one\n- two\n- three").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::List));
}

#[test]
fn parses_ordered_list_as_one_block() {
    let doc = read_md("1. one\n2. two\n3. three").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::List));
}

#[test]
fn list_text_concatenates_item_text() {
    let doc = read_md("- alpha\n- beta\n- gamma").expect("parse");
    let text = doc.paragraph_text(0).unwrap();
    for needle in ["alpha", "beta", "gamma"] {
        assert!(
            text.contains(needle),
            "list text missing {needle}: {text:?}"
        );
    }
}

#[test]
fn parses_nested_list_as_one_top_level_block() {
    let src = "- outer\n  - inner-a\n  - inner-b\n- outer-2";
    let doc = read_md(src).expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::List));
}

// ---------- Block quotes (CommonMark §5.1) ----------

#[test]
fn parses_single_line_block_quote() {
    let doc = read_md("> a quote").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::BlockQuote));
}

#[test]
fn parses_multi_line_block_quote() {
    let doc = read_md("> first line\n> second line").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::BlockQuote));
}

#[test]
fn block_quote_text_includes_inner_text() {
    let doc = read_md("> wisdom").expect("parse");
    assert!(doc.paragraph_text(0).unwrap().contains("wisdom"));
}

// ---------- Code blocks (CommonMark §4.4-4.5) ----------

#[test]
fn parses_fenced_code_block() {
    let doc = read_md("```\nhello world\n```").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::CodeBlock));
}

#[test]
fn parses_fenced_code_block_with_language() {
    let doc = read_md("```rust\nlet x = 1;\n```").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::CodeBlock));
}

#[test]
fn parses_indented_code_block() {
    // Four-space indent makes it a code block.
    let doc = read_md("    indented\n    code").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::CodeBlock));
}

#[test]
fn code_block_preserves_inner_text() {
    let doc = read_md("```\nlet x = 42;\n```").expect("parse");
    assert!(doc.paragraph_text(0).unwrap().contains("let x = 42;"));
}

// ---------- Thematic breaks (CommonMark §4.1) ----------

#[test]
fn parses_thematic_break_dashes() {
    let doc = read_md("---").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::ThematicBreak));
}

#[test]
fn parses_thematic_break_asterisks() {
    let doc = read_md("***").expect("parse");
    assert_eq!(doc.block_kind(0), Some(BlockKind::ThematicBreak));
}

#[test]
fn parses_thematic_break_underscores() {
    let doc = read_md("___").expect("parse");
    assert_eq!(doc.block_kind(0), Some(BlockKind::ThematicBreak));
}

// ---------- Multiple-block documents ----------

#[test]
fn parses_heading_then_paragraph_then_list() {
    let src = "# Title\n\nIntro paragraph.\n\n- item one\n- item two";
    let doc = read_md(src).expect("parse");
    assert_eq!(doc.paragraph_count(), 3);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(1)));
    assert_eq!(doc.block_kind(1), Some(BlockKind::Paragraph));
    assert_eq!(doc.block_kind(2), Some(BlockKind::List));
}

#[test]
fn parses_heading_paragraph_quote_code_break() {
    let src = "# H\n\np text.\n\n> quoted\n\n```\ncode\n```\n\n---";
    let doc = read_md(src).expect("parse");
    assert_eq!(doc.paragraph_count(), 5);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(1)));
    assert_eq!(doc.block_kind(1), Some(BlockKind::Paragraph));
    assert_eq!(doc.block_kind(2), Some(BlockKind::BlockQuote));
    assert_eq!(doc.block_kind(3), Some(BlockKind::CodeBlock));
    assert_eq!(doc.block_kind(4), Some(BlockKind::ThematicBreak));
}

// ---------- Edge cases ----------

#[test]
fn empty_input_yields_zero_blocks() {
    let doc = read_md("").expect("parse");
    assert_eq!(doc.paragraph_count(), 0);
}

#[test]
fn whitespace_only_input_yields_zero_blocks() {
    let doc = read_md("   \n\n\n  \n").expect("parse");
    assert_eq!(doc.paragraph_count(), 0);
}

#[test]
fn paragraph_text_out_of_bounds_returns_none() {
    let doc = read_md("p").expect("parse");
    assert!(doc.paragraph_text(99).is_none());
}

#[test]
fn block_kind_out_of_bounds_returns_none() {
    let doc = read_md("p").expect("parse");
    assert!(doc.block_kind(99).is_none());
}

#[test]
fn source_returns_input_verbatim() {
    let original = "# Title\n\nbody\n";
    let doc = read_md(original).expect("parse");
    assert_eq!(doc.source(), original);
}

// ---------- LATAM / multibyte ----------

#[test]
fn handles_latam_diacritics_in_paragraph_text() {
    let doc = read_md("Año en España: niñez. Mañana será el día más cálido.").expect("parse");
    let text = doc.paragraph_text(0).unwrap();
    for needle in ['ñ', 'á', 'í', 'í'] {
        assert!(text.contains(needle), "missing {needle}");
    }
}

#[test]
fn handles_cjk_text() {
    let doc = read_md("# 标题\n\n段落文本。").expect("parse");
    assert_eq!(doc.paragraph_count(), 2);
    assert_eq!(doc.paragraph_text(0), Some("标题"));
}

// ---------- GFM: tables ----------

#[test]
fn parses_gfm_pipe_table() {
    let src = "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
    let doc = read_md(src).expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Table));
}

#[test]
fn gfm_table_text_includes_cell_content() {
    let src = "| name | age |\n|---|---|\n| Marche | 30 |";
    let doc = read_md(src).expect("parse");
    let text = doc.paragraph_text(0).unwrap();
    for needle in ["name", "age", "Marche", "30"] {
        assert!(text.contains(needle), "table text missing {needle}");
    }
}

// ---------- GFM: footnotes ----------

#[test]
fn parses_gfm_footnote_reference_as_inline_in_paragraph() {
    let src = "Body[^1].\n\n[^1]: footnote text";
    let doc = read_md(src).expect("parse");
    assert!(doc.paragraph_count() >= 1);
    // The body paragraph survives as a Paragraph block.
    assert_eq!(doc.block_kind(0), Some(BlockKind::Paragraph));
}

#[test]
fn parses_gfm_footnote_definition() {
    let src = "Body[^1].\n\n[^1]: footnote text";
    let doc = read_md(src).expect("parse");
    let kinds: Vec<_> = (0..doc.paragraph_count())
        .filter_map(|i| doc.block_kind(i))
        .collect();
    assert!(
        kinds.contains(&BlockKind::FootnoteDefinition),
        "expected one FootnoteDefinition block, got {kinds:?}"
    );
}

// ---------- GFM: task lists ----------

#[test]
fn parses_gfm_task_list_as_list_block() {
    let src = "- [ ] todo\n- [x] done";
    let doc = read_md(src).expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::List));
}

#[test]
fn task_list_text_includes_item_text() {
    let src = "- [ ] write tests\n- [x] ship feature";
    let doc = read_md(src).expect("parse");
    let text = doc.paragraph_text(0).unwrap();
    assert!(text.contains("write tests"));
    assert!(text.contains("ship feature"));
}

// ---------- GFM: strikethrough ----------

#[test]
fn parses_gfm_strikethrough_as_inline_in_paragraph() {
    let doc = read_md("This is ~~struck~~ text.").expect("parse");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Paragraph));
    assert!(doc.paragraph_text(0).unwrap().contains("struck"));
}

// ---------- Property test: never panic ----------

#[cfg(test)]
mod prop {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        /// `read_md` must never panic on any UTF-8 input. Constrained
        /// to printable ASCII + a handful of markdown-significant
        /// punctuation + LATAM diacritics so the property exercises
        /// the parser's branches without spending the cases budget on
        /// junk bytes.
        #[test]
        fn prop_read_md_never_panics(
            s in r"[\sa-zA-Z0-9 #*_\-`>!\[\](){}|.,;:!\?\+\\/áéíóúüñçÁÉÍÓÚÜÑÇ]{0,200}",
        ) {
            let _ = read_md(&s);
        }

        /// On any input that parses, the editable surface must be
        /// internally consistent: every `i in 0..paragraph_count`
        /// resolves via both `paragraph_text` and `block_kind`.
        #[test]
        fn prop_indices_resolve(
            s in r"[\sa-zA-Z0-9 #*_\-`>!\[\](){}|.,;:!\?\+\\/]{0,200}",
        ) {
            if let Ok(doc) = read_md(&s) {
                let n = doc.paragraph_count();
                for i in 0..n {
                    prop_assert!(doc.paragraph_text(i).is_some());
                    prop_assert!(doc.block_kind(i).is_some());
                }
            }
        }
    }
}
