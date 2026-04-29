//! Round-trip tests: `read_md → write_md → read_md` produces an
//! equivalent `DocModel` on the editable surface (paragraph_count,
//! block_kind per index, paragraph_text per index).
//!
//! Plus a stronger byte-equivalence assertion for unmodified
//! parses: `write_md(read_md(s)) == s`. This enforces the
//! "lossless by default" contract the structural model promises
//! through its verbatim source.

use apalabrar_format_md::{read_md, write_md};
use proptest::prelude::*;

const FIXTURES: &[(&str, &str)] = &[
    ("empty", ""),
    ("single_paragraph", "Hello world."),
    ("two_paragraphs", "First.\n\nSecond.\n"),
    ("h1_then_paragraph", "# Title\n\nBody.\n"),
    (
        "all_heading_levels",
        "# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6\n",
    ),
    ("unordered_list", "- one\n- two\n- three\n"),
    ("ordered_list", "1. one\n2. two\n3. three\n"),
    (
        "nested_list",
        "- outer\n  - inner-a\n  - inner-b\n- outer-2\n",
    ),
    ("block_quote", "> quoted line\n"),
    ("fenced_code_block", "```rust\nfn main() {}\n```\n"),
    ("indented_code_block", "    let x = 1;\n    let y = 2;\n"),
    ("thematic_break", "---\n"),
    ("inline_emphasis", "Plain *italic* and **bold**.\n"),
    ("inline_code", "Use `printf`.\n"),
    ("inline_link", "See [docs](https://example.com).\n"),
    ("inline_image", "![alt](image.png)\n"),
    ("latam_diacritics", "Año mañana, día cálido. ¿Cómo está?\n"),
    ("cjk_heading_and_body", "# 标题\n\n段落文本。\n"),
    (
        "gfm_pipe_table",
        "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n",
    ),
    ("gfm_footnote", "Body[^1].\n\n[^1]: footnote text\n"),
    ("gfm_task_list", "- [ ] todo\n- [x] done\n"),
    ("gfm_strikethrough", "This is ~~struck~~ text.\n"),
    (
        "mixed_document",
        "# Title\n\nIntro.\n\n- item one\n- item two\n\n> quoted\n\n```\ncode\n```\n\n---\n",
    ),
];

#[test]
fn write_md_emits_source_verbatim_for_every_fixture() {
    for (label, src) in FIXTURES {
        let doc = read_md(src).expect("read_md");
        let out = write_md(&doc).expect("write_md");
        assert_eq!(
            out, *src,
            "{label}: write_md output diverged from input source"
        );
    }
}

#[test]
fn round_trip_preserves_paragraph_count_for_every_fixture() {
    for (label, src) in FIXTURES {
        let a = read_md(src).expect("read_md a");
        let out = write_md(&a).expect("write_md");
        let b = read_md(&out).expect("read_md b");
        assert_eq!(
            a.paragraph_count(),
            b.paragraph_count(),
            "{label}: paragraph_count drifted across round-trip"
        );
    }
}

#[test]
fn round_trip_preserves_block_kinds_for_every_fixture() {
    for (label, src) in FIXTURES {
        let a = read_md(src).expect("read_md a");
        let out = write_md(&a).expect("write_md");
        let b = read_md(&out).expect("read_md b");
        for i in 0..a.paragraph_count() {
            assert_eq!(
                a.block_kind(i),
                b.block_kind(i),
                "{label}: block_kind({i}) drifted across round-trip"
            );
        }
    }
}

#[test]
fn round_trip_preserves_paragraph_text_for_every_fixture() {
    for (label, src) in FIXTURES {
        let a = read_md(src).expect("read_md a");
        let out = write_md(&a).expect("write_md");
        let b = read_md(&out).expect("read_md b");
        for i in 0..a.paragraph_count() {
            assert_eq!(
                a.paragraph_text(i),
                b.paragraph_text(i),
                "{label}: paragraph_text({i}) drifted across round-trip"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    /// Property: for any input `read_md` accepts, the
    /// `read → write → read` cycle preserves paragraph_count and
    /// per-index block_kind / paragraph_text.
    #[test]
    fn prop_round_trip_preserves_editable_surface(
        s in r"[\sa-zA-Z0-9 #*_\-`>!\[\](){}|.,;:!\?\+\\/]{0,200}",
    ) {
        let Ok(a) = read_md(&s) else { return Ok(()); };
        let Ok(out) = write_md(&a) else { return Ok(()); };
        let Ok(b) = read_md(&out) else { return Ok(()); };
        prop_assert_eq!(a.paragraph_count(), b.paragraph_count());
        for i in 0..a.paragraph_count() {
            prop_assert_eq!(a.block_kind(i), b.block_kind(i));
            prop_assert_eq!(a.paragraph_text(i), b.paragraph_text(i));
        }
    }
}
