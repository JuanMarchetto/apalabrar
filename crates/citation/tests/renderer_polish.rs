//! Phase 5.2-polish-2 — targeted tests using custom CSL XML to
//! exercise renderer branches that the 10 bundled styles don't
//! reach (ordinal day suffix, plural editor/translator labels,
//! variable_present from `<if>`, resolve_variable arms for
//! page-first / ISBN / edition / abstract / id / type, and the
//! evaluate_conditions position arms).
//!
//! Strategy: parse a small CSL fixture via `parse_style`, build
//! a `RenderContext` directly, render, and assert on the
//! serialized HTML. This bypasses the bundled-style cache and
//! lets us probe one branch at a time.

use apalabrar_citation::ast::*;
use apalabrar_citation::ir::serialize;
use apalabrar_citation::parser::parse_style;
use apalabrar_citation::renderer::{RenderContext, render_layout};
use apalabrar_citation::{CslItem, DateVar, Html, NameVar};

fn empty_locale() -> Locale {
    Locale {
        lang: None,
        style_options: StyleOptions::default(),
        terms: Vec::new(),
        date_text: None,
        date_numeric: None,
    }
}

fn render_bib_xml(item: &CslItem, css: &str) -> String {
    let style = parse_style(css, "test").expect("style parses");
    let locale = empty_locale();
    let layout = style
        .bibliography
        .as_ref()
        .map(|b| &b.layout)
        .unwrap_or(&style.citation.layout);
    let mut ctx = RenderContext {
        style: &style,
        locale: &locale,
        item,
        position: 1,
        macro_depth: 0,
        is_bibliography: true,
        group_scopes: Vec::new(),
    };
    let tokens = render_layout(&mut ctx, layout);
    serialize(&tokens, &Html)
}

fn render_inline_xml(item: &CslItem, css: &str, position: usize) -> String {
    let style = parse_style(css, "test").expect("style parses");
    let locale = empty_locale();
    let mut ctx = RenderContext {
        style: &style,
        locale: &locale,
        item,
        position,
        macro_depth: 0,
        is_bibliography: false,
        group_scopes: Vec::new(),
    };
    let tokens = render_layout(&mut ctx, &style.citation.layout);
    serialize(&tokens, &Html)
}

fn item_with_date(year: i32, month: i32, day: i32) -> CslItem {
    CslItem {
        id: "x".into(),
        item_type: "article-journal".into(),
        title: Some("T".into()),
        author: vec![NameVar {
            family: Some("S".into()),
            given: Some("J".into()),
            non_dropping_particle: None,
            dropping_particle: None,
            suffix: None,
            literal: None,
        }],
        issued: Some(DateVar {
            date_parts: vec![vec![year, month, day]],
            literal: None,
            circa: false,
        }),
        ..CslItem::default()
    }
}

// ─────────────────────────────────────────────────────────────────
// english_ordinal_suffix: <date-part name="day" form="ordinal"/>
// ─────────────────────────────────────────────────────────────────

const ORDINAL_DAY_CSL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography>
    <layout>
      <date variable="issued">
        <date-part name="day" form="ordinal"/>
      </date>
    </layout>
  </bibliography>
</style>"#;

#[test]
fn ordinal_day_first_emits_st() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 1), ORDINAL_DAY_CSL),
        "1st"
    );
}

#[test]
fn ordinal_day_second_emits_nd() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 2), ORDINAL_DAY_CSL),
        "2nd"
    );
}

#[test]
fn ordinal_day_third_emits_rd() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 3), ORDINAL_DAY_CSL),
        "3rd"
    );
}

#[test]
fn ordinal_day_fourth_emits_th() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 4), ORDINAL_DAY_CSL),
        "4th"
    );
}

#[test]
fn ordinal_day_eleventh_emits_th_not_st() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 11), ORDINAL_DAY_CSL),
        "11th"
    );
}

#[test]
fn ordinal_day_twelfth_emits_th_not_nd() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 12), ORDINAL_DAY_CSL),
        "12th"
    );
}

#[test]
fn ordinal_day_thirteenth_emits_th_not_rd() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 13), ORDINAL_DAY_CSL),
        "13th"
    );
}

#[test]
fn ordinal_day_twentyfirst_emits_st() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 21), ORDINAL_DAY_CSL),
        "21st"
    );
}

#[test]
fn ordinal_day_twentysecond_emits_nd() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 22), ORDINAL_DAY_CSL),
        "22nd"
    );
}

#[test]
fn ordinal_day_twentythird_emits_rd() {
    assert_eq!(
        render_bib_xml(&item_with_date(2020, 6, 23), ORDINAL_DAY_CSL),
        "23rd"
    );
}

// ─────────────────────────────────────────────────────────────────
// resolve_variable arms — page-first, ISBN, edition, abstract,
// id, citation-key, type
// ─────────────────────────────────────────────────────────────────

fn item_rich() -> CslItem {
    CslItem {
        id: "smith2020".into(),
        item_type: "book".into(),
        title: Some("Long Title Of A Book".into()),
        author: vec![NameVar {
            family: Some("Smith".into()),
            given: Some("J".into()),
            non_dropping_particle: None,
            dropping_particle: None,
            suffix: None,
            literal: None,
        }],
        page: Some("123-145".into()),
        edition: Some("2nd".into()),
        isbn: Some("978-0-12-345678-9".into()),
        abstract_: Some("This is the abstract.".into()),
        ..CslItem::default()
    }
}

#[test]
fn resolve_page_first_arm_extracts_first_page_number() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="page-first"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "123");
}

#[test]
fn resolve_isbn_arm_emits_isbn_string() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="ISBN"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "978-0-12-345678-9");
}

#[test]
fn resolve_edition_arm_emits_edition_string() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="edition"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "2nd");
}

#[test]
fn resolve_abstract_arm_emits_abstract_text() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="abstract"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "This is the abstract.");
}

#[test]
fn resolve_id_arm_emits_item_id() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="id"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "smith2020");
}

#[test]
fn resolve_citation_key_arm_emits_item_id() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="citation-key"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "smith2020");
}

#[test]
fn resolve_type_arm_emits_item_type() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="type"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "book");
}

// ─────────────────────────────────────────────────────────────────
// render_text — variable="title" arm (mutation: delete arm)
// ─────────────────────────────────────────────────────────────────

#[test]
fn text_variable_title_emits_title_text() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout><text variable="title"/></layout></bibliography>
</style>"#;
    assert_eq!(render_bib_xml(&item_rich(), css), "Long Title Of A Book");
}

// ─────────────────────────────────────────────────────────────────
// variable_present via <if variable="..."/> condition
// ─────────────────────────────────────────────────────────────────

fn item_with_two_editors() -> CslItem {
    CslItem {
        id: "x".into(),
        item_type: "book".into(),
        editor: vec![
            NameVar {
                family: Some("Bloggs".into()),
                given: None,
                non_dropping_particle: None,
                dropping_particle: None,
                suffix: None,
                literal: None,
            },
            NameVar {
                family: Some("Jones".into()),
                given: None,
                non_dropping_particle: None,
                dropping_particle: None,
                suffix: None,
                literal: None,
            },
        ],
        ..CslItem::default()
    }
}

fn item_with_one_translator() -> CslItem {
    CslItem {
        id: "x".into(),
        item_type: "book".into(),
        translator: vec![NameVar {
            family: Some("Translator".into()),
            given: None,
            non_dropping_particle: None,
            dropping_particle: None,
            suffix: None,
            literal: None,
        }],
        ..CslItem::default()
    }
}

#[test]
fn if_variable_author_present_renders_branch() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout>
    <choose>
      <if variable="author"><text value="HAS_AUTHOR"/></if>
      <else><text value="NO_AUTHOR"/></else>
    </choose>
  </layout></bibliography>
</style>"#;
    let with = render_bib_xml(&item_rich(), css);
    let mut item_no_auth = item_rich();
    item_no_auth.author.clear();
    let without = render_bib_xml(&item_no_auth, css);
    assert_eq!(with, "HAS_AUTHOR");
    assert_eq!(without, "NO_AUTHOR");
}

#[test]
fn if_variable_editor_present_renders_branch() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout>
    <choose>
      <if variable="editor"><text value="HAS_EDITOR"/></if>
      <else><text value="NO_EDITOR"/></else>
    </choose>
  </layout></bibliography>
</style>"#;
    let with = render_bib_xml(&item_with_two_editors(), css);
    let without = render_bib_xml(&item_rich(), css);
    assert_eq!(with, "HAS_EDITOR");
    assert_eq!(without, "NO_EDITOR");
}

#[test]
fn if_variable_translator_present_renders_branch() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <bibliography><layout>
    <choose>
      <if variable="translator"><text value="HAS_TR"/></if>
      <else><text value="NO_TR"/></else>
    </choose>
  </layout></bibliography>
</style>"#;
    let with = render_bib_xml(&item_with_one_translator(), css);
    let without = render_bib_xml(&item_rich(), css);
    assert_eq!(with, "HAS_TR");
    assert_eq!(without, "NO_TR");
}

// ─────────────────────────────────────────────────────────────────
// evaluate_conditions — position="first" / "subsequent"
// ─────────────────────────────────────────────────────────────────

#[test]
fn position_first_matches_when_position_is_one() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <citation><layout>
    <choose>
      <if position="first"><text value="FIRST"/></if>
      <else><text value="OTHER"/></else>
    </choose>
  </layout></citation>
</style>"#;
    assert_eq!(render_inline_xml(&item_rich(), css, 1), "FIRST");
}

#[test]
fn position_subsequent_matches_when_position_above_one() {
    let css = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <citation><layout>
    <choose>
      <if position="subsequent"><text value="SUBS"/></if>
      <else><text value="FIRST"/></else>
    </choose>
  </layout></citation>
</style>"#;
    assert_eq!(render_inline_xml(&item_rich(), css, 2), "SUBS");
}
