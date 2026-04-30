//! Phase 5.2-impl/parser — smoke tests on the 5 bundled styles + 8
//! locales. Goal: every bundled XML parses without error.

use apalabrar_citation::ast::StyleClass;
use apalabrar_citation::parser::{parse_locale, parse_style};

const APA_XML: &str = include_str!("../assets/styles/apa.csl");
const IEEE_XML: &str = include_str!("../assets/styles/ieee.csl");
const MLA_XML: &str = include_str!("../assets/styles/mla.csl");
const AMA_XML: &str = include_str!("../assets/styles/ama.csl");
const CHICAGO_XML: &str = include_str!("../assets/styles/chicago-notes-bibliography.csl");

const EN_US: &str = include_str!("../assets/locales/locales-en-US.xml");
const EN_GB: &str = include_str!("../assets/locales/locales-en-GB.xml");
const ES_ES: &str = include_str!("../assets/locales/locales-es-ES.xml");
const PT_BR: &str = include_str!("../assets/locales/locales-pt-BR.xml");
const DE_DE: &str = include_str!("../assets/locales/locales-de-DE.xml");
const FR_FR: &str = include_str!("../assets/locales/locales-fr-FR.xml");
const ZH_CN: &str = include_str!("../assets/locales/locales-zh-CN.xml");
const RU_RU: &str = include_str!("../assets/locales/locales-ru-RU.xml");

#[test]
fn parse_apa_succeeds() {
    let s = parse_style(APA_XML, "apa").expect("apa parse");
    assert_eq!(s.id, "apa");
    assert!(s.bibliography.is_some(), "apa has a bibliography section");
    assert!(!s.macros.is_empty(), "apa defines macros");
    assert_eq!(s.class, StyleClass::InText);
}

#[test]
fn parse_ieee_succeeds() {
    let s = parse_style(IEEE_XML, "ieee").expect("ieee parse");
    assert_eq!(s.id, "ieee");
    assert!(s.bibliography.is_some());
    assert_eq!(s.class, StyleClass::InText);
}

#[test]
fn parse_mla_succeeds() {
    let s = parse_style(MLA_XML, "mla").expect("mla parse");
    assert_eq!(s.id, "mla");
    assert!(s.bibliography.is_some());
}

#[test]
fn parse_ama_succeeds() {
    let s = parse_style(AMA_XML, "ama").expect("ama parse");
    assert_eq!(s.id, "ama");
    assert!(s.bibliography.is_some());
}

#[test]
fn parse_chicago_notes_bibliography_is_note_class() {
    let s = parse_style(CHICAGO_XML, "chicago-notes-bibliography").expect("chicago parse");
    assert_eq!(s.class, StyleClass::Note);
    assert!(s.bibliography.is_some());
}

#[test]
fn parse_en_us_locale_succeeds() {
    let l = parse_locale(EN_US).expect("en-US parse");
    assert_eq!(l.lang.as_deref(), Some("en-US"));
    assert!(!l.terms.is_empty(), "en-US must define terms");
    assert!(l.date_text.is_some(), "en-US must define text date");
    assert!(l.date_numeric.is_some(), "en-US must define numeric date");
}

#[test]
fn all_8_bundled_locales_parse() {
    for (name, xml) in [
        ("en-US", EN_US),
        ("en-GB", EN_GB),
        ("es-ES", ES_ES),
        ("pt-BR", PT_BR),
        ("de-DE", DE_DE),
        ("fr-FR", FR_FR),
        ("zh-CN", ZH_CN),
        ("ru-RU", RU_RU),
    ] {
        let l = parse_locale(xml).unwrap_or_else(|e| panic!("{name} parse failed: {e:?}"));
        assert_eq!(
            l.lang.as_deref(),
            Some(name),
            "lang attr round-trip for {name}"
        );
        assert!(!l.terms.is_empty(), "{name} must define terms");
    }
}

#[test]
fn locale_terms_carry_known_term_names() {
    let l = parse_locale(EN_US).unwrap();
    let names: std::collections::HashSet<&str> = l.terms.iter().map(|t| t.name.as_str()).collect();
    // Sanity-check a few canonical terms that any locale must define.
    assert!(names.contains("and"), "en-US must define 'and'");
    assert!(names.contains("editor"), "en-US must define 'editor'");
    assert!(names.contains("no date"), "en-US must define 'no date'");
}

#[test]
fn parse_unknown_xml_returns_err() {
    let err = parse_style("<not-csl/>", "x").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("missing root") || msg.contains("style"));
}
