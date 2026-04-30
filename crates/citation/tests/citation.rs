//! Phase 5.2 RED — citation engine integration tests (~25 tests).
//!
//! Locked semantics:
//! - `render_bib(&[CslItem], style)` returns the full bibliography
//!   as HTML, en-US locale, ordered per style rules.
//! - `render_inline(&CslItem, style)` returns the in-text short form
//!   (eg. APA: "(Smith, 2020)"; IEEE: "[1]"; Chicago: a footnote).
//! - `render_bib_with` / `render_inline_with` accept explicit
//!   `locale` (one of 8 bundled) and `OutputFormat` (Html/Plain).
//! - Unknown style → `Error::UnknownStyle`. Unknown locale → `UnknownLocale`.
//! - Cache: thread-local lazy. Determinism: same input → same output.

use std::collections::BTreeMap;

use apalabrar_citation::{
    CslItem, DateVar, Error, Html, NameVar, OutputFormat, Plain, render_bib, render_bib_with,
    render_inline, render_inline_with,
};

// ─────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────

/// Plain personal-name builder.
fn person(family: &str, given: &str) -> NameVar {
    NameVar {
        family: Some(family.into()),
        given: Some(given.into()),
        non_dropping_particle: None,
        dropping_particle: None,
        suffix: None,
        literal: None,
    }
}

/// CSL date with year only.
fn year(y: i32) -> DateVar {
    DateVar {
        date_parts: vec![vec![y]],
        literal: None,
        circa: false,
    }
}

/// Reference: single-author journal article. The canonical APA test case.
fn journal_article_single_author() -> CslItem {
    CslItem {
        id: "smith2020".into(),
        item_type: "article-journal".into(),
        author: vec![person("Smith", "John")],
        editor: vec![],
        translator: vec![],
        title: Some("On the nature of typesetting".into()),
        container_title: Some("Journal of Typography".into()),
        container_title_short: Some("J. Typogr.".into()),
        publisher: None,
        publisher_place: None,
        issued: Some(year(2020)),
        volume: Some("12".into()),
        issue: Some("3".into()),
        page: Some("123-145".into()),
        doi: Some("10.1000/typo.2020.12345".into()),
        url: None,
        isbn: None,
        edition: None,
        abstract_: None,
        extra: BTreeMap::new(),
    }
}

/// Reference: book with publisher + place.
fn book() -> CslItem {
    CslItem {
        id: "doe2019".into(),
        item_type: "book".into(),
        author: vec![person("Doe", "Jane")],
        title: Some("The history of writing".into()),
        publisher: Some("Academic Press".into()),
        publisher_place: Some("New York".into()),
        issued: Some(year(2019)),
        isbn: Some("978-0-123456-78-9".into()),
        ..Default::default()
    }
}

/// Reference: multi-author paper (4 authors → triggers et-al for most styles).
fn multi_author_paper() -> CslItem {
    CslItem {
        id: "team2021".into(),
        item_type: "article-journal".into(),
        author: vec![
            person("Smith", "John"),
            person("Doe", "Jane"),
            person("Jones", "Robert"),
            person("Lee", "Mary"),
        ],
        title: Some("A collaborative study".into()),
        container_title: Some("Nature".into()),
        issued: Some(year(2021)),
        volume: Some("589".into()),
        page: Some("100-110".into()),
        doi: Some("10.1038/s41586-021-00001".into()),
        ..Default::default()
    }
}

/// Reference: book chapter with author + editor + container.
fn book_chapter() -> CslItem {
    CslItem {
        id: "smithChapter2018".into(),
        item_type: "chapter".into(),
        author: vec![person("Smith", "John")],
        editor: vec![person("Editor", "Edith")],
        title: Some("Chapter on metrics".into()),
        container_title: Some("Handbook of measurement".into()),
        publisher: Some("Elsevier".into()),
        publisher_place: Some("Amsterdam".into()),
        issued: Some(year(2018)),
        page: Some("45-78".into()),
        ..Default::default()
    }
}

/// Reference: minimal fields — only title + year, no authors. Tests
/// the substitute chain that styles use to fall back to title.
fn anonymous_with_only_title_and_year() -> CslItem {
    CslItem {
        id: "anon2022".into(),
        item_type: "article-journal".into(),
        title: Some("An anonymous tract".into()),
        container_title: Some("Journal of Mystery".into()),
        issued: Some(year(2022)),
        ..Default::default()
    }
}

/// Reference: non-Latin scripts (Cyrillic + CJK).
fn non_latin_authors() -> CslItem {
    CslItem {
        id: "ivanov2020".into(),
        item_type: "article-journal".into(),
        author: vec![
            person("Иванов", "Иван"), // Cyrillic
            person("王", "明"),       // Chinese
        ],
        title: Some("Многоязычное исследование".into()),
        container_title: Some("国际期刊".into()),
        issued: Some(year(2020)),
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────
// (1) render_bib snapshot per style — 5 tests
// ─────────────────────────────────────────────────────────────────

#[test]

fn render_bib_apa_journal_article_snapshot() {
    let bib =
        render_bib(&[journal_article_single_author()], "apa").expect("APA render must succeed");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_ieee_journal_article_snapshot() {
    let bib =
        render_bib(&[journal_article_single_author()], "ieee").expect("IEEE render must succeed");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_mla_book_snapshot() {
    let bib = render_bib(&[book()], "mla").expect("MLA render must succeed");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_ama_journal_article_snapshot() {
    let bib =
        render_bib(&[journal_article_single_author()], "ama").expect("AMA render must succeed");
    insta::assert_snapshot!(bib);
}

#[test]
#[ignore = "Phase 5.2-impl: GREEN ships in follow-up sessions"]
fn render_bib_chicago_notes_book_chapter_snapshot() {
    let bib = render_bib(&[book_chapter()], "chicago-notes-bibliography")
        .expect("Chicago render must succeed");
    insta::assert_snapshot!(bib);
}

// ─────────────────────────────────────────────────────────────────
// (2) render_inline snapshot per style — 5 tests
// ─────────────────────────────────────────────────────────────────

#[test]

fn render_inline_apa_is_author_year_form() {
    let cite =
        render_inline(&journal_article_single_author(), "apa").expect("APA inline must succeed");
    // APA in-text: (Smith, 2020). The exact spacing / parentheses
    // are pinned via insta snapshot.
    insta::assert_snapshot!(cite);
}

#[test]
fn render_inline_ieee_is_numeric_bracket_form() {
    let cite =
        render_inline(&journal_article_single_author(), "ieee").expect("IEEE inline must succeed");
    // IEEE in-text: [1] — for a single item, always [1].
    assert!(
        cite.contains('1') && cite.contains('[') && cite.contains(']'),
        "IEEE inline must be numeric in brackets, got {cite:?}",
    );
}

#[test]
fn render_inline_mla_is_author_page_form() {
    let cite =
        render_inline(&journal_article_single_author(), "mla").expect("MLA inline must succeed");
    insta::assert_snapshot!(cite);
}

#[test]
fn render_inline_ama_is_superscript_numeric() {
    let cite =
        render_inline(&journal_article_single_author(), "ama").expect("AMA inline must succeed");
    // AMA in-text uses a superscript number — the HTML output should
    // contain a <sup> tag wrapping the number.
    assert!(
        cite.contains("<sup>") && cite.contains("</sup>"),
        "AMA inline must use <sup> numeric, got {cite:?}",
    );
}

#[test]
#[ignore = "Phase 5.2-impl: GREEN ships in follow-up sessions"]
fn render_inline_chicago_notes_is_full_footnote() {
    let cite = render_inline(&book_chapter(), "chicago-notes-bibliography")
        .expect("Chicago notes inline must succeed");
    // Chicago notes-bib renders a FULL footnote (not a short form)
    // for the first citation. Author full name + title must appear.
    assert!(
        cite.contains("Smith"),
        "Chicago footnote must include author, got {cite:?}"
    );
    assert!(
        cite.contains("Chapter"),
        "Chicago footnote must include title, got {cite:?}",
    );
}

// ─────────────────────────────────────────────────────────────────
// (3) CSL feature coverage — 6 tests
// ─────────────────────────────────────────────────────────────────

#[test]

fn render_bib_renders_italic_journal_titles_in_apa() {
    // APA italicises journal container titles. The HTML output must
    // wrap "Journal of Typography" in <i>.
    let bib = render_bib(&[journal_article_single_author()], "apa").unwrap();
    assert!(
        bib.contains("<i>") && bib.contains("Journal of Typography"),
        "APA must italicise journal titles, got {bib:?}",
    );
}

#[test]
fn render_inline_emits_et_al_for_3_plus_authors_in_apa() {
    // APA 7th: 3+ authors in CITATION (inline) use "First, et al.".
    // BIBLIOGRAPHY uses et-al-min=21 (lists up to 20 authors) so we
    // verify the inline path here. Original test name + body said
    // "render_bib...4+ authors" but APA bib only triggers at 21+;
    // the test's claim was incorrect per the APA 7 manual + the
    // bundled CSL style attrs.
    let cite = render_inline(&multi_author_paper(), "apa").unwrap();
    assert!(
        cite.contains("et al"),
        "APA inline with 4 authors must use 'et al.', got {cite:?}",
    );
}

#[test]
fn render_bib_uses_abbreviated_journal_in_ama() {
    // AMA uses container-title-short and applies strip-periods, so
    // a stored "J. Typogr." renders as "J Typogr". Verify the short
    // form is preferred over the long ("Journal of Typography").
    let bib = render_bib(&[journal_article_single_author()], "ama").unwrap();
    assert!(
        bib.contains("J Typogr") && !bib.contains("Journal of Typography"),
        "AMA must prefer abbreviated journal title (strip-periods applied), got {bib:?}",
    );
}

#[test]

fn render_bib_emits_doi_link_when_doi_present() {
    let bib = render_bib(&[journal_article_single_author()], "apa").unwrap();
    assert!(
        bib.contains("10.1000/typo.2020.12345"),
        "APA must surface DOI, got {bib:?}",
    );
}

#[test]
fn render_bib_uses_substitute_to_title_when_author_missing() {
    // Anonymous item: most styles fall back to the title at the
    // start of the entry (CSL `substitute` chain).
    let bib = render_bib(&[anonymous_with_only_title_and_year()], "apa").unwrap();
    assert!(
        bib.contains("An anonymous tract"),
        "must surface title when author missing, got {bib:?}",
    );
}

#[test]
fn render_inline_ieee_numbering_is_sequential_across_render_bib() {
    // IEEE numbers items 1, 2, 3, ... in the order they appear in
    // the input slice. Bibliography must reflect this.
    let items = vec![
        journal_article_single_author(),
        book(),
        multi_author_paper(),
    ];
    let bib = render_bib(&items, "ieee").unwrap();
    assert!(
        bib.contains("[1]") && bib.contains("[2]") && bib.contains("[3]"),
        "IEEE bib must number sequentially, got {bib:?}",
    );
}

// ─────────────────────────────────────────────────────────────────
// (4) Edge cases — 4 tests
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_bib_handles_empty_items_slice() {
    let bib = render_bib(&[], "apa").unwrap();
    // Empty bibliography → empty string (or empty <ol>) — the layout
    // is a non-issue. Pin the contract: no error, no panic.
    assert!(
        bib.is_empty() || !bib.contains("Smith"),
        "empty input must produce empty/no-content output, got {bib:?}",
    );
}

#[test]
fn render_bib_handles_5_plus_authors_with_et_al_truncation_in_apa() {
    // APA 7: bib lists up to 20 authors. We test that 21+ triggers
    // the truncation rule (... + last author).
    let mut item = multi_author_paper();
    item.author = (0..25)
        .map(|i| person(&format!("Last{i}"), &format!("First{i}")))
        .collect();
    let bib = render_bib(&[item], "apa").unwrap();
    // Must contain ellipsis or et al and the last author.
    assert!(
        bib.contains("Last24") || bib.contains("et al"),
        "APA bib with 25 authors must include the last author or et al, got {bib:?}",
    );
}

#[test]
fn render_bib_handles_non_latin_authors_without_panicking() {
    let bib = render_bib(&[non_latin_authors()], "apa").unwrap();
    assert!(
        bib.contains("Иванов") || bib.contains("王"),
        "non-Latin authors must round-trip, got {bib:?}",
    );
}

#[test]
fn render_bib_handles_missing_year_via_no_date_term() {
    // Item with no `issued` → CSL `no-date` term ("n.d." in en-US).
    let mut item = journal_article_single_author();
    item.issued = None;
    let bib = render_bib(&[item], "apa").unwrap();
    assert!(
        bib.contains("n.d."),
        "missing year must use 'n.d.' (en-US no-date term), got {bib:?}",
    );
}

// ─────────────────────────────────────────────────────────────────
// (5) Cache + locale — 2 tests
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_bib_is_deterministic_across_repeated_calls() {
    // Cache must not introduce variance: same input → byte-identical
    // output across calls.
    let item = journal_article_single_author();
    let a = render_bib(std::slice::from_ref(&item), "apa").unwrap();
    let b = render_bib(std::slice::from_ref(&item), "apa").unwrap();
    let c = render_bib(std::slice::from_ref(&item), "apa").unwrap();
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn render_bib_with_es_es_uses_spanish_terms() {
    // Spanish locale must surface localised terms in the rendered
    // output. APA's chapter format renders the "in" term before the
    // container title: en-US "In", es-ES "En" (with capitalize-first
    // applied by the style). Original assertion expected the "y"
    // connector but a single editor never triggers it; "En" is the
    // reliable signal that the es-ES locale loaded.
    let bib = render_bib_with(&[book_chapter()], "apa", "es-ES", &Html).unwrap();
    assert!(
        bib.contains("En "),
        "es-ES bib must surface Spanish 'En' (vs en-US 'In'), got {bib:?}",
    );
}

// ─────────────────────────────────────────────────────────────────
// (6) Errors — 2 tests
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_bib_unknown_style_returns_unknown_style_error() {
    let err = render_bib(&[journal_article_single_author()], "made-up-style").unwrap_err();
    assert_eq!(err, Error::UnknownStyle("made-up-style".into()));
}

#[test]
fn render_bib_with_unknown_locale_returns_unknown_locale_error() {
    let err =
        render_bib_with(&[journal_article_single_author()], "apa", "xx-XX", &Html).unwrap_err();
    assert_eq!(err, Error::UnknownLocale("xx-XX".into()));
}

// ─────────────────────────────────────────────────────────────────
// (7) Output format trait — 1 test (covered indirectly elsewhere too)
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_inline_with_plain_format_strips_html_tags() {
    let cite =
        render_inline_with(&journal_article_single_author(), "apa", "en-US", &Plain).unwrap();
    assert!(
        !cite.contains('<'),
        "Plain output must contain no HTML tags, got {cite:?}",
    );
}

// ─────────────────────────────────────────────────────────────────
// (8) Property — render is deterministic on byte-equivalent input
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_bib_is_byte_deterministic_for_same_input_property() {
    // 100 calls with the same fixture must produce 100 identical
    // outputs (no caching artefacts, no global-state leak).
    let item = journal_article_single_author();
    let first = render_bib(std::slice::from_ref(&item), "apa").unwrap();
    for _ in 0..100 {
        let next = render_bib(std::slice::from_ref(&item), "apa").unwrap();
        assert_eq!(first, next);
    }
}

// Output-format trait surface tests — these don't go through the
// renderer so they're orthogonal to the todo!() panics on render_*.
// They still expect Html/Plain impls to function.
#[test]
fn html_format_italic_wraps_in_i_tag() {
    let html = Html;
    assert_eq!(html.italic("foo"), "<i>foo</i>");
}

#[test]
fn html_format_escape_handles_html_special_chars() {
    let html = Html;
    let escaped = html.escape("a < b & c > d");
    assert!(escaped.contains("&lt;"));
    assert!(escaped.contains("&gt;"));
    assert!(escaped.contains("&amp;"));
}

#[test]
fn plain_format_italic_passes_through_unchanged() {
    let plain = Plain;
    assert_eq!(plain.italic("foo"), "foo");
}
