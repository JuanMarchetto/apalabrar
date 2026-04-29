//! HTML element coverage + sanitization + round-trip tests for
//! `read_html` / `write_html`.
//!
//! Three pillars:
//!
//! 1. Element classification: each recognised top-level block kind
//!    (Paragraph, Heading 1-6, List, BlockQuote, CodeBlock, Table,
//!    ThematicBreak) lands in the right bucket with the right text.
//!
//! 2. Sanitization on save: `write_html` only emits tags from the
//!    controlled subset, regardless of what the input carried.
//!    Scripts, styles, classes, inline event handlers, and arbitrary
//!    attributes never appear in the output.
//!
//! 3. Round-trip semantics: `read_html(write_html(read_html(s)))`
//!    produces a `DocModel` equivalent on the editable surface
//!    (paragraph_count + per-index block_kind + paragraph_text).

use apalabrar_format_html::{BlockKind, read_html, write_html};
use proptest::prelude::*;

// ---------- Headings (h1-h6) ----------

#[test]
fn parses_h1_through_h6_as_heading_blocks_with_correct_levels() {
    let src = "<h1>One</h1><h2>Two</h2><h3>Three</h3><h4>Four</h4><h5>Five</h5><h6>Six</h6>";
    let doc = read_html(src).expect("read");
    assert_eq!(doc.paragraph_count(), 6);
    for (i, lvl) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6)] {
        assert_eq!(doc.block_kind(i), Some(BlockKind::Heading(lvl)));
    }
}

#[test]
fn h1_text_is_extracted_verbatim() {
    let doc = read_html("<h1>Hello</h1>").expect("read");
    assert_eq!(doc.paragraph_text(0), Some("Hello"));
}

#[test]
fn heading_with_inline_emphasis_concatenates_text() {
    let doc = read_html("<h2>Plain <em>italic</em> word</h2>").expect("read");
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(2)));
    let t = doc.paragraph_text(0).unwrap();
    for needle in ["Plain", "italic", "word"] {
        assert!(t.contains(needle), "missing {needle}");
    }
}

// ---------- Paragraphs ----------

#[test]
fn parses_single_paragraph() {
    let doc = read_html("<p>Hello world.</p>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Paragraph));
    assert_eq!(doc.paragraph_text(0), Some("Hello world."));
}

#[test]
fn parses_two_paragraphs() {
    let doc = read_html("<p>First.</p><p>Second.</p>").expect("read");
    assert_eq!(doc.paragraph_count(), 2);
    assert_eq!(doc.paragraph_text(0), Some("First."));
    assert_eq!(doc.paragraph_text(1), Some("Second."));
}

#[test]
fn paragraph_with_strong_and_em_concatenates_inline_runs() {
    let doc =
        read_html("<p>Plain <strong>bold</strong> and <em>italic</em> text.</p>").expect("read");
    let t = doc.paragraph_text(0).unwrap();
    for needle in ["Plain", "bold", "italic", "text"] {
        assert!(t.contains(needle), "missing {needle}");
    }
}

#[test]
fn paragraph_with_anchor_includes_link_text() {
    let doc =
        read_html(r#"<p>See <a href="https://example.com">docs</a> for more.</p>"#).expect("read");
    let t = doc.paragraph_text(0).unwrap();
    assert!(t.contains("docs"));
    assert!(t.contains("more"));
}

#[test]
fn paragraph_with_inline_code_includes_code_text() {
    let doc = read_html("<p>Use <code>printf</code> for output.</p>").expect("read");
    let t = doc.paragraph_text(0).unwrap();
    assert!(t.contains("printf"));
}

// ---------- Lists ----------

#[test]
fn parses_unordered_list_as_one_block() {
    let doc = read_html("<ul><li>one</li><li>two</li><li>three</li></ul>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::List));
}

#[test]
fn parses_ordered_list_as_one_block() {
    let doc = read_html("<ol><li>one</li><li>two</li></ol>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::List));
}

#[test]
fn list_text_concatenates_item_text() {
    let doc = read_html("<ul><li>alpha</li><li>beta</li><li>gamma</li></ul>").expect("read");
    let t = doc.paragraph_text(0).unwrap();
    for needle in ["alpha", "beta", "gamma"] {
        assert!(t.contains(needle), "missing {needle}");
    }
}

// ---------- Block quote ----------

#[test]
fn parses_blockquote_as_block_quote() {
    let doc = read_html("<blockquote><p>quoted wisdom</p></blockquote>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::BlockQuote));
}

#[test]
fn blockquote_text_includes_inner_text() {
    let doc = read_html("<blockquote>quoted wisdom</blockquote>").expect("read");
    assert!(doc.paragraph_text(0).unwrap().contains("wisdom"));
}

// ---------- Code block ----------

#[test]
fn parses_pre_as_code_block() {
    let doc = read_html("<pre>line1\nline2</pre>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::CodeBlock));
}

#[test]
fn parses_pre_with_code_as_code_block() {
    let doc = read_html("<pre><code>let x = 42;</code></pre>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::CodeBlock));
}

#[test]
fn code_block_preserves_inner_text() {
    let doc = read_html("<pre>let x = 42;</pre>").expect("read");
    assert!(doc.paragraph_text(0).unwrap().contains("let x = 42;"));
}

// ---------- Table ----------

#[test]
fn parses_table_as_one_block() {
    let src =
        "<table><tr><th>name</th><th>age</th></tr><tr><td>Marche</td><td>30</td></tr></table>";
    let doc = read_html(src).expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Table));
}

#[test]
fn table_text_includes_cell_content() {
    let src = "<table><tr><td>foo</td><td>bar</td></tr></table>";
    let doc = read_html(src).expect("read");
    let t = doc.paragraph_text(0).unwrap();
    assert!(t.contains("foo"));
    assert!(t.contains("bar"));
}

// ---------- Thematic break ----------

#[test]
fn parses_hr_as_thematic_break() {
    let doc = read_html("<hr>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::ThematicBreak));
}

// ---------- Multi-block document ----------

#[test]
fn parses_heading_paragraph_list_quote_pre_hr_in_order() {
    let src = "<h1>Title</h1><p>Intro.</p><ul><li>a</li><li>b</li></ul>\
               <blockquote>q</blockquote><pre>code</pre><hr>";
    let doc = read_html(src).expect("read");
    assert_eq!(doc.paragraph_count(), 6);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Heading(1)));
    assert_eq!(doc.block_kind(1), Some(BlockKind::Paragraph));
    assert_eq!(doc.block_kind(2), Some(BlockKind::List));
    assert_eq!(doc.block_kind(3), Some(BlockKind::BlockQuote));
    assert_eq!(doc.block_kind(4), Some(BlockKind::CodeBlock));
    assert_eq!(doc.block_kind(5), Some(BlockKind::ThematicBreak));
}

// ---------- Pasted-HTML noise tolerance ----------

#[test]
fn tolerates_class_and_style_attributes() {
    let src = r#"<p class="MsoNormal" style="margin:0">Body text</p>"#;
    let doc = read_html(src).expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.block_kind(0), Some(BlockKind::Paragraph));
    assert_eq!(doc.paragraph_text(0), Some("Body text"));
}

#[test]
fn tolerates_data_attributes_and_role() {
    let src = r#"<p data-x="y" role="text">Body</p>"#;
    let doc = read_html(src).expect("read");
    assert_eq!(doc.paragraph_text(0), Some("Body"));
}

#[test]
fn ignores_script_tag_content() {
    // Script content must NOT contribute to any paragraph_text.
    let src = "<p>Visible</p><script>alert('xss')</script>";
    let doc = read_html(src).expect("read");
    let combined: String = (0..doc.paragraph_count())
        .filter_map(|i| doc.paragraph_text(i))
        .collect::<Vec<_>>()
        .join("|");
    assert!(combined.contains("Visible"));
    assert!(
        !combined.contains("alert"),
        "script content leaked into model: {combined:?}"
    );
}

#[test]
fn ignores_style_tag_content() {
    let src = "<style>p{color:red}</style><p>Body</p>";
    let doc = read_html(src).expect("read");
    let combined: String = (0..doc.paragraph_count())
        .filter_map(|i| doc.paragraph_text(i))
        .collect::<Vec<_>>()
        .join("|");
    assert!(combined.contains("Body"));
    assert!(!combined.contains("color:red"));
}

#[test]
fn ignores_nested_script_inside_paragraph() {
    // Script content nested INSIDE a paragraph (rather than as a
    // sibling) must still be filtered. Catches mutations on the
    // script/style filtering branches inside `walk_text` — without
    // the script-skip the inner "alert" leaks into paragraph_text;
    // without the Element arm the wildcard branch recurses into
    // every element including script.
    let doc = read_html("<p>before<script>alert('xss')</script>after</p>").expect("read");
    let t = doc.paragraph_text(0).unwrap();
    assert!(t.contains("before"), "missing 'before' in {t:?}");
    assert!(t.contains("after"), "missing 'after' in {t:?}");
    assert!(
        !t.contains("alert"),
        "nested script content leaked into paragraph_text: {t:?}"
    );
}

#[test]
fn ignores_nested_style_inside_paragraph() {
    let doc = read_html("<p>visible<style>p{color:red}</style>text</p>").expect("read");
    let t = doc.paragraph_text(0).unwrap();
    assert!(t.contains("visible"));
    assert!(t.contains("text"));
    assert!(
        !t.contains("color:red"),
        "nested style content leaked into paragraph_text: {t:?}"
    );
}

#[test]
fn empty_script_alone_does_not_create_paragraph() {
    // With `script || style` correctly classifying both as None,
    // a doc that's only a script produces 0 blocks. With the
    // disjunction broken to `&&`, the script falls through to the
    // catch-all Paragraph and we get a stray (empty) block.
    let doc = read_html("<script>alert('xss')</script>").expect("read");
    assert_eq!(
        doc.paragraph_count(),
        0,
        "lone script element produced unexpected blocks"
    );
}

#[test]
fn empty_style_alone_does_not_create_paragraph() {
    let doc = read_html("<style>p{color:red}</style>").expect("read");
    assert_eq!(doc.paragraph_count(), 0);
}

#[test]
fn script_sibling_of_paragraph_does_not_inflate_block_count() {
    // When script is a body sibling of a paragraph (not just inside
    // <head>), classify_top_level must return None so we get exactly
    // ONE paragraph block, not two. Catches the
    // `script || style → script && style` mutation in
    // classify_top_level — under that mutation the script falls
    // through to the Paragraph catch-all and a stray empty block
    // appears.
    let doc = read_html("<p>Visible</p><script>alert('xss')</script>").expect("read");
    assert_eq!(
        doc.paragraph_count(),
        1,
        "expected exactly one block; script must filter, not classify as Paragraph"
    );
    assert_eq!(doc.paragraph_text(0), Some("Visible"));
}

#[test]
fn style_sibling_of_paragraph_does_not_inflate_block_count() {
    let doc = read_html("<p>Visible</p><style>p{color:red}</style>").expect("read");
    assert_eq!(doc.paragraph_count(), 1);
    assert_eq!(doc.paragraph_text(0), Some("Visible"));
}

// ---------- Multibyte ----------

#[test]
fn handles_latam_diacritics() {
    let doc = read_html("<p>Año cálido el día más frío en España</p>").expect("read");
    let t = doc.paragraph_text(0).unwrap();
    for needle in ['ñ', 'á', 'í', 'í'] {
        assert!(t.contains(needle), "missing {needle}");
    }
}

#[test]
fn handles_cjk_text() {
    let doc = read_html("<h1>标题</h1><p>段落文本。</p>").expect("read");
    assert_eq!(doc.paragraph_count(), 2);
    assert_eq!(doc.paragraph_text(0), Some("标题"));
}

// ---------- Edge cases ----------

#[test]
fn empty_input_yields_zero_blocks() {
    let doc = read_html("").expect("read");
    assert_eq!(doc.paragraph_count(), 0);
}

#[test]
fn whitespace_only_input_yields_zero_blocks() {
    let doc = read_html("   \n\n  \t").expect("read");
    assert_eq!(doc.paragraph_count(), 0);
}

#[test]
fn paragraph_text_out_of_bounds_returns_none() {
    let doc = read_html("<p>x</p>").expect("read");
    assert!(doc.paragraph_text(99).is_none());
}

#[test]
fn block_kind_out_of_bounds_returns_none() {
    let doc = read_html("<p>x</p>").expect("read");
    assert!(doc.block_kind(99).is_none());
}

// ---------- write_html: controlled subset ----------

#[test]
fn write_html_emits_p_for_paragraph_block() {
    let doc = read_html("<p>Body</p>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<p>"), "expected <p> in {out:?}");
    assert!(out.contains("</p>"));
    assert!(out.contains("Body"));
}

#[test]
fn write_html_emits_h1_for_h1_block() {
    let doc = read_html("<h1>Title</h1>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<h1>"));
    assert!(out.contains("</h1>"));
    assert!(out.contains("Title"));
}

#[test]
fn write_html_emits_h6_for_h6_block() {
    let doc = read_html("<h6>Tiny</h6>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<h6>"));
    assert!(out.contains("</h6>"));
}

#[test]
fn write_html_strips_class_and_style_attributes() {
    let src = r#"<p class="MsoNormal" style="margin:0" data-x="y">Body</p>"#;
    let doc = read_html(src).expect("read");
    let out = write_html(&doc).expect("write");
    assert!(
        !out.contains("class="),
        "class= leaked into output: {out:?}"
    );
    assert!(!out.contains("style="));
    assert!(!out.contains("data-"));
}

#[test]
fn write_html_strips_script_tags_via_omission() {
    let src = "<p>Visible</p><script>alert('xss')</script>";
    let doc = read_html(src).expect("read");
    let out = write_html(&doc).expect("write");
    assert!(!out.contains("<script"));
    assert!(!out.contains("alert"));
}

#[test]
fn write_html_strips_style_tags_via_omission() {
    let src = "<style>p{color:red}</style><p>Body</p>";
    let doc = read_html(src).expect("read");
    let out = write_html(&doc).expect("write");
    assert!(!out.contains("<style"));
    assert!(!out.contains("color:red"));
}

#[test]
fn write_html_emits_ul_with_li_for_list_block() {
    let doc = read_html("<ul><li>one</li><li>two</li></ul>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<ul>"));
    assert!(out.contains("<li>"));
    assert!(out.contains("</li>"));
    assert!(out.contains("</ul>"));
}

#[test]
fn write_html_emits_blockquote() {
    let doc = read_html("<blockquote>quote</blockquote>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<blockquote>"));
    assert!(out.contains("</blockquote>"));
}

#[test]
fn write_html_emits_pre_for_code_block() {
    let doc = read_html("<pre>let x = 1;</pre>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<pre>"));
    assert!(out.contains("</pre>"));
    assert!(out.contains("let x = 1;"));
}

#[test]
fn write_html_emits_hr_for_thematic_break() {
    let doc = read_html("<hr>").expect("read");
    let out = write_html(&doc).expect("write");
    assert!(out.contains("<hr"), "expected <hr in {out:?}");
}

#[test]
fn write_html_escapes_html_special_characters_in_text() {
    let doc = read_html("<p>x &lt; y &amp;&amp; a &gt; b</p>").expect("read");
    let out = write_html(&doc).expect("write");
    // After parse, the model holds decoded text "x < y && a > b".
    // After write, the special chars must be re-escaped to &lt; &amp; &gt;.
    assert!(out.contains("&lt;"), "missing &lt; in {out:?}");
    assert!(out.contains("&amp;"));
    assert!(out.contains("&gt;"));
    assert!(
        !out.contains("x < y"),
        "raw '<' / '>' leaked into output: {out:?}"
    );
}

// ---------- Round-trip semantics ----------

const ROUND_TRIP_FIXTURES: &[(&str, &str)] = &[
    ("h1", "<h1>Title</h1>"),
    ("h2", "<h2>Sub</h2>"),
    ("h3", "<h3>Three</h3>"),
    ("h4", "<h4>Four</h4>"),
    ("h5", "<h5>Five</h5>"),
    ("h6", "<h6>Six</h6>"),
    ("p", "<p>Body.</p>"),
    ("two_paragraphs", "<p>One.</p><p>Two.</p>"),
    ("ul", "<ul><li>a</li><li>b</li></ul>"),
    ("ol", "<ol><li>a</li><li>b</li></ol>"),
    ("blockquote", "<blockquote>q</blockquote>"),
    ("pre", "<pre>code</pre>"),
    ("table", "<table><tr><td>a</td><td>b</td></tr></table>"),
    ("hr", "<hr>"),
    (
        "mixed",
        "<h1>T</h1><p>p</p><ul><li>x</li></ul><blockquote>q</blockquote><pre>c</pre><hr>",
    ),
    (
        "noisy_paragraph",
        r#"<p class="MsoNormal" style="m:0">Body</p>"#,
    ),
];

#[test]
fn round_trip_preserves_paragraph_count() {
    for (label, src) in ROUND_TRIP_FIXTURES {
        let a = read_html(src).expect("read a");
        let out = write_html(&a).expect("write");
        let b = read_html(&out).expect("read b");
        assert_eq!(
            a.paragraph_count(),
            b.paragraph_count(),
            "{label}: paragraph_count drifted"
        );
    }
}

#[test]
fn round_trip_preserves_block_kinds() {
    for (label, src) in ROUND_TRIP_FIXTURES {
        let a = read_html(src).expect("read a");
        let out = write_html(&a).expect("write");
        let b = read_html(&out).expect("read b");
        for i in 0..a.paragraph_count() {
            assert_eq!(
                a.block_kind(i),
                b.block_kind(i),
                "{label}: block_kind({i}) drifted"
            );
        }
    }
}

#[test]
fn round_trip_preserves_paragraph_text() {
    for (label, src) in ROUND_TRIP_FIXTURES {
        let a = read_html(src).expect("read a");
        let out = write_html(&a).expect("write");
        let b = read_html(&out).expect("read b");
        for i in 0..a.paragraph_count() {
            assert_eq!(
                a.paragraph_text(i),
                b.paragraph_text(i),
                "{label}: paragraph_text({i}) drifted"
            );
        }
    }
}

// ---------- Property tests ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// `read_html` must never panic on any UTF-8 input. Regex
    /// constrains to printable ASCII + a few HTML-significant
    /// punctuation characters + LATAM diacritics so the property
    /// exercises real branches without spending the case budget on
    /// junk bytes.
    #[test]
    fn prop_read_html_never_panics(
        s in r"[\sa-zA-Z0-9 <>/=\-_\.,;:'\?\!áéíóúüñçÁÉÍÓÚÜÑÇ]{0,200}",
    ) {
        let _ = read_html(&s);
    }

    /// On any input that parses, every `i in 0..paragraph_count`
    /// resolves via both `paragraph_text` and `block_kind`.
    #[test]
    fn prop_indices_resolve(
        s in r"[\sa-zA-Z0-9 <>/=\-_\.,;:]{0,200}",
    ) {
        if let Ok(doc) = read_html(&s) {
            let n = doc.paragraph_count();
            for i in 0..n {
                prop_assert!(doc.paragraph_text(i).is_some());
                prop_assert!(doc.block_kind(i).is_some());
            }
        }
    }

    /// Round-trip property: `read → write → read` preserves
    /// paragraph_count + block_kind + paragraph_text.
    #[test]
    fn prop_round_trip_preserves_editable_surface(
        s in r"[\sa-zA-Z0-9 <>/=\-_\.,;:]{0,200}",
    ) {
        let Ok(a) = read_html(&s) else { return Ok(()); };
        let Ok(out) = write_html(&a) else { return Ok(()); };
        let Ok(b) = read_html(&out) else { return Ok(()); };
        prop_assert_eq!(a.paragraph_count(), b.paragraph_count());
        for i in 0..a.paragraph_count() {
            prop_assert_eq!(a.block_kind(i), b.block_kind(i));
            prop_assert_eq!(a.paragraph_text(i), b.paragraph_text(i));
        }
    }
}
