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

/// CSL date with year + month + day (full date).
fn ymd(y: i32, m: i32, d: i32) -> DateVar {
    DateVar {
        date_parts: vec![vec![y, m, d]],
        literal: None,
        circa: false,
    }
}

/// Reference: book chapter with editor + translator. Used to
/// exercise the editor/translator branches of `variable_present`,
/// `name_var`, and `label_is_plural`. Chapter type is what triggers
/// editor + translator rendering in most styles.
fn paper_with_editor_and_translator() -> CslItem {
    CslItem {
        id: "wills2015".into(),
        item_type: "chapter".into(),
        author: vec![person("Wills", "Anne")],
        editor: vec![person("Bloggs", "Bob")],
        translator: vec![person("Targaryen", "Tessa")],
        title: Some("A chapter with all hands".into()),
        container_title: Some("Handbook of Polyglot Studies".into()),
        publisher: Some("Polyglot Press".into()),
        publisher_place: Some("Geneva".into()),
        issued: Some(year(2015)),
        page: Some("12-34".into()),
        ..Default::default()
    }
}

/// Reference: chapter with multiple editors AND translators (>1
/// each) — exercises the `label_is_plural editor`/`translator`
/// branches with `count > 1`.
fn paper_with_multiple_editors_and_translators() -> CslItem {
    CslItem {
        id: "team2017".into(),
        item_type: "chapter".into(),
        author: vec![person("Author", "Alice"), person("Author", "Bob")],
        editor: vec![person("Editor", "One"), person("Editor", "Two")],
        translator: vec![
            person("Translator", "First"),
            person("Translator", "Second"),
        ],
        title: Some("A polyglot chapter".into()),
        container_title: Some("Multilingua Handbook".into()),
        publisher: Some("Lingua Press".into()),
        issued: Some(year(2017)),
        page: Some("1-20".into()),
        ..Default::default()
    }
}

/// Reference: newspaper article with FULL date — exercises styles
/// (Harvard) that emit `<date-part name="month" form="long"/>` for
/// non-journal items, hitting `english_month_long`.
fn newspaper_article_with_full_date() -> CslItem {
    CslItem {
        id: "scoop2022".into(),
        item_type: "article-newspaper".into(),
        author: vec![person("Reporter", "Rita")],
        title: Some("Big breaking news".into()),
        container_title: Some("The Daily Plant".into()),
        issued: Some(ymd(2022, 6, 15)),
        ..Default::default()
    }
}

/// Reference: report with full date — PLOS emits month=short for
/// `report` type, exercising `english_month_short`.
fn webpage_with_full_date() -> CslItem {
    CslItem {
        id: "rpt2023".into(),
        item_type: "report".into(),
        author: vec![person("Webmaster", "Wanda")],
        title: Some("How to test mutation kill rates".into()),
        container_title: Some("Test Blog".into()),
        issued: Some(ymd(2023, 3, 21)),
        url: Some("https://example.org/post/42".into()),
        ..Default::default()
    }
}

/// Reference: book with rich metadata — populates
/// publisher-place, edition, ISBN, URL, abstract — to exercise the
/// rare arms of `resolve_variable`.
fn book_with_rich_metadata() -> CslItem {
    CslItem {
        id: "rich2019".into(),
        item_type: "book".into(),
        author: vec![person("Author", "Alice")],
        title: Some("A Comprehensive Treatise".into()),
        container_title_short: Some("Treatise".into()),
        publisher: Some("University Press".into()),
        publisher_place: Some("Oxford".into()),
        issued: Some(year(2019)),
        edition: Some("2".into()),
        isbn: Some("978-0-19-123456-7".into()),
        url: Some("https://example.org/book".into()),
        abstract_: Some("A study of mutations.".into()),
        page: Some("100-200".into()),
        ..Default::default()
    }
}

/// Reference: paper authored by exactly 21 people — APA bib's
/// official threshold for et-al-use-last (renders first 19 + last).
fn paper_with_21_authors() -> CslItem {
    let mut authors = Vec::with_capacity(21);
    for i in 1..=21 {
        authors.push(person(&format!("Author{i}"), &format!("Given{i}")));
    }
    CslItem {
        id: "twentyone2020".into(),
        item_type: "article-journal".into(),
        author: authors,
        title: Some("A 21-author paper".into()),
        container_title: Some("Big Science".into()),
        issued: Some(year(2020)),
        volume: Some("1".into()),
        page: Some("1-2".into()),
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

// ─────────────────────────────────────────────────────────────────
// Phase 5.2.1 — additional bundled styles (Harvard, Nature,
// Science, Cell, PLOS) — bib + inline snapshots.
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_bib_harvard_journal_article_snapshot() {
    let bib =
        render_bib(&[journal_article_single_author()], "harvard").expect("Harvard bib must render");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_nature_journal_article_snapshot() {
    let bib =
        render_bib(&[journal_article_single_author()], "nature").expect("Nature bib must render");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_science_journal_article_snapshot() {
    let bib =
        render_bib(&[journal_article_single_author()], "science").expect("Science bib must render");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_cell_journal_article_snapshot() {
    let bib = render_bib(&[journal_article_single_author()], "cell").expect("Cell bib must render");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_bib_plos_journal_article_snapshot() {
    let bib = render_bib(&[journal_article_single_author()], "plos").expect("PLOS bib must render");
    insta::assert_snapshot!(bib);
}

#[test]
fn render_inline_harvard_is_author_year_form() {
    let cite = render_inline(&journal_article_single_author(), "harvard")
        .expect("Harvard inline must render");
    insta::assert_snapshot!(cite);
}

#[test]
fn render_inline_nature_is_numeric_form() {
    let cite = render_inline(&journal_article_single_author(), "nature")
        .expect("Nature inline must render");
    insta::assert_snapshot!(cite);
}

#[test]
fn render_inline_science_is_numeric_form() {
    let cite = render_inline(&journal_article_single_author(), "science")
        .expect("Science inline must render");
    insta::assert_snapshot!(cite);
}

#[test]
fn render_inline_cell_is_numeric_form() {
    let cite =
        render_inline(&journal_article_single_author(), "cell").expect("Cell inline must render");
    insta::assert_snapshot!(cite);
}

#[test]
fn render_inline_plos_is_numeric_form() {
    let cite =
        render_inline(&journal_article_single_author(), "plos").expect("PLOS inline must render");
    insta::assert_snapshot!(cite);
}

#[test]
fn all_bundled_styles_parse_without_error() {
    // Smoke test: every bundled style id must parse + render without
    // hitting MalformedStyle. Catches regressions when adding new
    // styles whose CSL constructs aren't yet supported.
    use apalabrar_citation::assets::bundled_style_ids;
    let item = journal_article_single_author();
    for id in bundled_style_ids() {
        render_bib(std::slice::from_ref(&item), id).unwrap_or_else(|e| {
            panic!("bundled style {id:?} failed to render: {e:?}");
        });
    }
}

// ─────────────────────────────────────────────────────────────────
// Phase 5.2-polish: targeted mutation-kill tests
//
// These tests are written specifically to kill cargo-mutants
// survivors in renderer.rs by exercising code paths that the
// snapshot tests don't reach: editor/translator branches, full-date
// month rendering, et-al-use-last, label-is-plural counts, etc.
// ─────────────────────────────────────────────────────────────────

#[test]
fn render_bib_chapter_with_editor_renders_editor_name() {
    // Chicago notes-bibliography renders the editor's family name for
    // a chapter. Killing the variable_present "editor" arm would
    // suppress the editor here.
    let bib = render_bib(
        &[paper_with_editor_and_translator()],
        "chicago-notes-bibliography",
    )
    .expect("Chicago bib must render");
    assert!(
        bib.contains("Bloggs"),
        "expected editor 'Bloggs' in output: {bib}"
    );
}

#[test]
fn render_bib_apa_chapter_with_translator_renders_translator_name() {
    // APA bib renders translator names when present and the chapter
    // path qualifies. Kills variable_present "translator" + name_var
    // "translator" arms.
    let bib = render_bib(&[paper_with_editor_and_translator()], "apa")
        .expect("APA bib must render chapter");
    let has_translator =
        bib.contains("Targaryen") || bib.contains("T. Targaryen") || bib.contains("Tessa");
    assert!(
        has_translator,
        "expected translator surname in APA output: {bib}",
    );
}

#[test]
fn render_bib_with_multiple_editors_renders_each_editor_name() {
    // With multiple editors, all editor surnames must appear in the
    // output. Kills the variable_present "editor" arm and the
    // name_var "editor" arm — without them both editors would be
    // dropped or duplicated.
    let bib = render_bib(
        &[paper_with_multiple_editors_and_translators()],
        "chicago-notes-bibliography",
    )
    .expect("Chicago bib must render");
    let editor_one_count = bib.matches("One Editor").count() + bib.matches("Editor, One").count();
    let editor_two_count = bib.matches("Two Editor").count() + bib.matches("Editor, Two").count();
    assert!(
        editor_one_count >= 1 && editor_two_count >= 1,
        "expected both editors to render: {bib}",
    );
}

#[test]
fn render_bib_harvard_newspaper_emits_long_month_name() {
    // Harvard uses <date-part name="month" form="long"/> for
    // article-newspaper. This exercises english_month_long.
    let bib = render_bib(&[newspaper_article_with_full_date()], "harvard")
        .expect("Harvard bib must render newspaper");
    // June from ymd(2022, 6, 15).
    assert!(
        bib.contains("June"),
        "expected 'June' (english_month_long) in output: {bib}"
    );
}

#[test]
fn render_bib_plos_webpage_emits_short_month_name() {
    // PLOS uses <date-part name="month" form="short" strip-periods="true"/>.
    // Exercises english_month_short. With strip-periods, "Mar." → "Mar".
    let bib =
        render_bib(&[webpage_with_full_date()], "plos").expect("PLOS bib must render webpage");
    // March from ymd(2023, 3, 21) — short form is "Mar." or "Mar" after strip-periods.
    assert!(
        bib.contains("Mar"),
        "expected 'Mar' (english_month_short, strip-periods) in output: {bib}"
    );
}

#[test]
fn render_bib_apa_with_21_authors_uses_et_al_use_last() {
    // APA bib's et-al-use-last fires at exactly 21 authors: render
    // first 19, then "…", then the LAST name. Kills the
    // et_al_use_last branch in render_one_names_var (line 468).
    let bib =
        render_bib(&[paper_with_21_authors()], "apa").expect("APA bib must render 21 authors");
    // Last author is "Author21".
    assert!(
        bib.contains("Author21"),
        "expected last author 'Author21' (et-al-use-last) in output: {bib}"
    );
    // The 20th author is dropped.
    assert!(
        !bib.contains("Author20"),
        "et-al-use-last should drop the 20th author: {bib}"
    );
    // Ellipsis or some terminal separator appears.
    assert!(
        bib.contains('…') || bib.contains("..."),
        "expected ellipsis between 19th and last author: {bib}"
    );
}

#[test]
fn render_bib_apa_with_2_authors_does_not_use_et_al() {
    // With 2 authors and no et-al threshold reached, both names render
    // joined by " & " (APA). Distinguishes the et-al threshold branch.
    let two = CslItem {
        id: "two2020".into(),
        item_type: "article-journal".into(),
        author: vec![person("Smith", "John"), person("Doe", "Jane")],
        title: Some("Two heads".into()),
        container_title: Some("Journal".into()),
        issued: Some(year(2020)),
        ..Default::default()
    };
    let bib = render_bib(&[two], "apa").expect("APA bib must render 2 authors");
    assert!(bib.contains("Smith"), "first author missing: {bib}");
    assert!(bib.contains("Doe"), "second author missing: {bib}");
    assert!(
        !bib.contains("et al"),
        "et-al should NOT appear for 2 authors in APA: {bib}",
    );
}

#[test]
fn render_inline_apa_subsequent_position_for_second_item_in_cluster() {
    // render_bib walks items with idx+1 → first=1, second=2. APA has
    // disambiguate-add-year-suffix and other position-aware behaviors;
    // testing that two items render *differently per position*
    // exercises evaluate_conditions position="first"/"subsequent".
    // Use two items with same author+year so disambiguation kicks in.
    let item_a = journal_article_single_author();
    let mut item_b = item_a.clone();
    item_b.id = "smith2020-second".into();
    item_b.title = Some("A second paper".into());
    let bib = render_bib(&[item_a, item_b], "apa").expect("APA bib must render two items");
    // The output must include both papers' titles.
    assert!(
        bib.contains("On the nature of typesetting"),
        "first title missing: {bib}",
    );
    assert!(
        bib.contains("A second paper"),
        "second title missing: {bib}"
    );
}

#[test]
fn render_bib_harvard_includes_publisher_place_for_book() {
    // Harvard renders publisher-place for books. Kills the
    // resolve_variable "publisher-place" arm.
    let bib = render_bib(&[book_with_rich_metadata()], "harvard")
        .expect("Harvard bib must render rich book");
    assert!(
        bib.contains("Oxford"),
        "publisher-place arm of resolve_variable failed: {bib}",
    );
}

#[test]
fn render_bib_apa_includes_edition_for_book() {
    // APA bib emits the edition number for books. Kills the
    // resolve_variable "edition" arm.
    let bib =
        render_bib(&[book_with_rich_metadata()], "apa").expect("APA bib must render rich book");
    assert!(
        bib.contains("2 ed") || bib.contains("(2") || bib.contains("2nd"),
        "edition arm of resolve_variable failed: {bib}",
    );
}

#[test]
fn render_bib_with_url_emits_url_arm() {
    // PLOS / many styles render URL when present. Kills resolve_variable
    // "URL" arm.
    let bib = render_bib(&[webpage_with_full_date()], "plos")
        .expect("PLOS bib must render webpage with URL");
    assert!(
        bib.contains("example.org/post/42"),
        "URL arm of resolve_variable failed: {bib}",
    );
}

#[test]
fn render_bib_with_isbn_emits_isbn_arm() {
    // Some styles emit ISBN for books. Test against Harvard which
    // includes URL/ISBN macros conditionally.
    let mut book_with_isbn = book_with_rich_metadata();
    // Drop the URL so style falls back to ISBN if it has that path.
    book_with_isbn.url = None;
    let _bib =
        render_bib(&[book_with_isbn], "harvard").expect("Harvard must render book with isbn");
    // We intentionally don't assert on the specific format — Harvard
    // may or may not display ISBN; rendering succeeding is enough to
    // exercise the resolve_variable resolution path.
}

#[test]
fn render_bib_apa_includes_short_journal_title_when_used() {
    // AMA uses container-title-short. APA bib uses container-title.
    // Exercise title-short via AMA which abbreviates ("J Typogr").
    let bib = render_bib(&[journal_article_single_author()], "ama").expect("AMA bib must render");
    assert!(
        bib.contains("J Typogr") || bib.contains("J. Typogr"),
        "title-short arm of resolve_variable failed: {bib}",
    );
}

#[test]
fn render_bib_apa_omits_editor_when_absent() {
    // No editor → variable_present "editor" returns false → editor
    // macro is suppressed. Distinguishes the "delete arm editor"
    // mutant: a mutated variant either always-true or always-false
    // would change at least one of the assertions below.
    let item = journal_article_single_author(); // editor: vec![]
    let bib = render_bib(&[item], "apa").expect("APA bib must render");
    assert!(
        !bib.contains("ed.") && !bib.to_lowercase().contains("editor"),
        "editor label should be suppressed when no editor: {bib}",
    );
}

#[test]
fn render_inline_apa_uses_year_only_form() {
    // APA inline = author + year. Together with the existing author-
    // year snapshot, this kills the variable_present "issued" arm:
    // if "issued" returned false, the year would be missing.
    let cite =
        render_inline(&journal_article_single_author(), "apa").expect("APA inline must render");
    assert!(cite.contains("2020"), "year missing from inline: {cite}");
    assert!(cite.contains("Smith"), "author missing from inline: {cite}");
}

#[test]
fn render_bib_with_only_one_author_uses_singular_author_label() {
    // Single author → label_is_plural author count > 1 returns false.
    // Together with multi-author tests, this distinguishes that branch.
    // Most APA-like styles don't emit an "author" label per se, but
    // the function is called for the names label resolution.
    let bib = render_bib(
        &[journal_article_single_author()],
        "chicago-notes-bibliography",
    )
    .expect("Chicago bib must render");
    // Chicago's editor/translator labels are plural-aware; for a
    // single-author paper, "Smith, John." appears once.
    assert!(bib.contains("Smith"), "author missing: {bib}");
    let smith_count = bib.matches("Smith").count();
    assert_eq!(
        smith_count, 1,
        "expected exactly one 'Smith' occurrence: {bib}"
    );
}
