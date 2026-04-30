//! Phase 5.2-impl/parser — focused unit tests on individual CSL
//! constructs to boost parser coverage to 95%+.

use apalabrar_citation::ast::*;
use apalabrar_citation::parser::{ParseError, parse_locale, parse_style};

fn wrap_style(citation_body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <citation>
    <layout>{citation_body}</layout>
  </citation>
</style>"#
    )
}

fn first_layout_elem(s: &Style) -> &Element {
    s.citation
        .layout
        .body
        .first()
        .expect("layout has at least one element")
}

// ─────────────────────────────────────────────────────────────────
// Public API errors
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_style_missing_root_returns_err() {
    let err = parse_style("<not-csl/>", "x").unwrap_err();
    matches!(err, ParseError::MissingRoot(_));
}

#[test]
fn parse_locale_missing_root_returns_err() {
    let err = parse_locale("<not-csl/>").unwrap_err();
    matches!(err, ParseError::MissingRoot(_));
}

#[test]
fn parse_style_with_macro_missing_name_attr_returns_err() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <macro>
    <text value="x"/>
  </macro>
</style>"#;
    let err = parse_style(xml, "x").unwrap_err();
    matches!(err, ParseError::MissingAttr { .. });
}

#[test]
fn parse_style_xml_error_returns_err() {
    let err = parse_style("<unterminated", "x").unwrap_err();
    matches!(err, ParseError::Xml(_));
}

// ─────────────────────────────────────────────────────────────────
// Style class
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_style_default_class_is_in_text() {
    let s = parse_style(&wrap_style(""), "x").unwrap();
    assert_eq!(s.class, StyleClass::InText);
}

#[test]
fn parse_style_explicit_in_text_class() {
    let xml =
        r#"<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0"></style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.class, StyleClass::InText);
}

#[test]
fn parse_style_default_locale_attr_round_trips() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" default-locale="es-ES"></style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.default_locale.as_deref(), Some("es-ES"));
}

// ─────────────────────────────────────────────────────────────────
// Text element source variants
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_text_with_variable_source() {
    let s = parse_style(&wrap_style(r#"<text variable="title"/>"#), "x").unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert!(matches!(&t.source, TextSource::Variable(v) if v == "title"));
}

#[test]
fn parse_text_with_macro_source() {
    let s = parse_style(&wrap_style(r#"<text macro="author"/>"#), "x").unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert!(matches!(&t.source, TextSource::Macro(m) if m == "author"));
}

#[test]
fn parse_text_with_term_source() {
    let s = parse_style(&wrap_style(r#"<text term="and" form="long"/>"#), "x").unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert!(matches!(&t.source, TextSource::Term(name) if name == "and"));
    assert_eq!(t.form.as_deref(), Some("long"));
}

#[test]
fn parse_text_with_value_source() {
    let s = parse_style(&wrap_style(r#"<text value="hello"/>"#), "x").unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert!(matches!(&t.source, TextSource::Value(v) if v == "hello"));
}

#[test]
fn parse_text_with_formatting_attrs() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-style="italic" font-weight="bold" prefix="(" suffix=")"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert_eq!(t.formatting.font_style, Some(FontStyle::Italic));
    assert_eq!(t.formatting.font_weight, Some(FontWeight::Bold));
    assert_eq!(t.prefix.as_deref(), Some("("));
    assert_eq!(t.suffix.as_deref(), Some(")"));
}

#[test]
fn parse_text_with_quotes_strip_periods_text_case() {
    let s = parse_style(
        &wrap_style(
            r#"<text variable="title" quotes="true" strip-periods="true" text-case="capitalize-all"/>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert!(t.quotes);
    assert!(t.strip_periods);
    assert_eq!(t.text_case, Some(TextCase::Capitalize));
}

// ─────────────────────────────────────────────────────────────────
// Number element forms
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_number_with_each_form() {
    for (attr, expected) in [
        ("numeric", NumberForm::Numeric),
        ("ordinal", NumberForm::Ordinal),
        ("long-ordinal", NumberForm::LongOrdinal),
        ("roman", NumberForm::Roman),
    ] {
        let s = parse_style(
            &wrap_style(&format!(r#"<number variable="volume" form="{attr}"/>"#)),
            "x",
        )
        .unwrap();
        let Element::Number(n) = first_layout_elem(&s) else {
            panic!("expected number");
        };
        assert_eq!(n.form, Some(expected), "form={attr}");
    }
}

// ─────────────────────────────────────────────────────────────────
// Group / Choose / Names / Date / Label
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_group_with_delimiter_and_body() {
    let s = parse_style(
        &wrap_style(
            r#"<group delimiter=", "><text variable="title"/><text variable="page"/></group>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Group(g) = first_layout_elem(&s) else {
        panic!("expected group");
    };
    assert_eq!(g.delimiter.as_deref(), Some(", "));
    assert_eq!(g.body.len(), 2);
}

#[test]
fn parse_choose_with_if_else_if_else() {
    let s = parse_style(
        &wrap_style(
            r#"<choose>
          <if variable="title"><text variable="title"/></if>
          <else-if variable="container-title"><text variable="container-title"/></else-if>
          <else><text value="anonymous"/></else>
        </choose>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Choose(c) = first_layout_elem(&s) else {
        panic!("expected choose");
    };
    assert_eq!(c.branches.len(), 2);
    assert_eq!(c.else_body.len(), 1);
}

#[test]
fn parse_choose_with_match_any() {
    let s = parse_style(
        &wrap_style(
            r#"<choose><if variable="title issued" match="any"><text value="x"/></if></choose>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Choose(c) = first_layout_elem(&s) else {
        panic!("expected choose");
    };
    assert_eq!(c.branches[0].conditions.match_kind, MatchKind::Any);
    assert_eq!(c.branches[0].conditions.variable.len(), 2);
}

#[test]
fn parse_choose_with_match_none() {
    let s = parse_style(
        &wrap_style(r#"<choose><if type="webpage" match="none"><text value="x"/></if></choose>"#),
        "x",
    )
    .unwrap();
    let Element::Choose(c) = first_layout_elem(&s) else {
        panic!("expected choose");
    };
    assert_eq!(c.branches[0].conditions.match_kind, MatchKind::None);
}

#[test]
fn parse_names_with_substitute_chain() {
    let s = parse_style(
        &wrap_style(
            r#"<names variable="author">
          <name form="long"/>
          <substitute>
            <names variable="editor"/>
            <text variable="title"/>
          </substitute>
        </names>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Names(n) = first_layout_elem(&s) else {
        panic!("expected names");
    };
    assert_eq!(n.variables, vec!["author"]);
    assert!(n.name.is_some());
    assert_eq!(n.substitute.len(), 2);
}

#[test]
fn parse_name_with_form_and_initialize() {
    let s = parse_style(
        &wrap_style(
            r#"<names variable="author"><name form="short" initialize-with=". " name-as-sort-order="first"/></names>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Names(n) = first_layout_elem(&s) else {
        panic!("expected names");
    };
    let nm = n.name.as_ref().unwrap();
    assert_eq!(nm.form, Some(NameForm::Short));
    assert_eq!(nm.initialize_with.as_deref(), Some(". "));
    assert_eq!(nm.name_as_sort_order, Some(NameAsSortOrder::First));
}

#[test]
fn parse_name_with_and_text_and_symbol() {
    for (val, expected) in [("text", NameAnd::Text), ("symbol", NameAnd::Symbol)] {
        let s = parse_style(
            &wrap_style(&format!(
                r#"<names variable="author"><name and="{val}"/></names>"#
            )),
            "x",
        )
        .unwrap();
        let Element::Names(n) = first_layout_elem(&s) else {
            panic!("expected names");
        };
        assert_eq!(n.name.as_ref().unwrap().and, Some(expected));
    }
}

#[test]
fn parse_name_with_name_part_children() {
    let s = parse_style(
        &wrap_style(
            r#"<names variable="author"><name><name-part name="family" font-weight="bold"/></name></names>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Names(n) = first_layout_elem(&s) else {
        panic!("expected names");
    };
    let parts = &n.name.as_ref().unwrap().name_parts;
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].name, "family");
    assert_eq!(parts[0].formatting.font_weight, Some(FontWeight::Bold));
}

#[test]
fn parse_date_with_inline_date_parts() {
    let s = parse_style(
        &wrap_style(
            r#"<date variable="issued">
          <date-part name="year" suffix=", "/>
          <date-part name="month" form="short"/>
          <date-part name="day" form="numeric-leading-zeros"/>
        </date>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Date(d) = first_layout_elem(&s) else {
        panic!("expected date");
    };
    assert_eq!(d.date_parts.len(), 3);
    assert_eq!(d.date_parts[0].name, DatePartName::Year);
    assert_eq!(d.date_parts[1].form, Some(DatePartForm::Short));
    assert_eq!(
        d.date_parts[2].form,
        Some(DatePartForm::NumericLeadingZeros)
    );
}

#[test]
fn parse_date_with_form_text_and_filter() {
    let s = parse_style(
        &wrap_style(r#"<date variable="issued" form="text" date-parts="year-month"/>"#),
        "x",
    )
    .unwrap();
    let Element::Date(d) = first_layout_elem(&s) else {
        panic!("expected date");
    };
    assert_eq!(d.form, Some(DateForm::Text));
    assert_eq!(d.date_parts_filter, Some(DatePartsFilter::YearMonth));
}

#[test]
fn parse_label_with_variable_and_plural() {
    let s = parse_style(
        &wrap_style(r#"<label variable="page" form="short" plural="contextual"/>"#),
        "x",
    )
    .unwrap();
    let Element::Label(l) = first_layout_elem(&s) else {
        panic!("expected label");
    };
    assert_eq!(l.variable.as_deref(), Some("page"));
    assert_eq!(l.form.as_deref(), Some("short"));
    assert_eq!(l.plural.as_deref(), Some("contextual"));
}

// ─────────────────────────────────────────────────────────────────
// Sort
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_citation_sort_with_variable_and_macro_keys() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <citation>
        <sort>
          <key variable="issued"/>
          <key macro="author" sort="descending"/>
        </sort>
        <layout/>
      </citation>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.citation.sort.len(), 2);
    assert!(matches!(&s.citation.sort[0].source, SortSource::Variable(v) if v == "issued"));
    assert!(matches!(&s.citation.sort[1].source, SortSource::Macro(m) if m == "author"));
    assert!(s.citation.sort[1].sort_descending);
}

// ─────────────────────────────────────────────────────────────────
// Bibliography
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_bibliography_with_full_attrs() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <citation><layout/></citation>
      <bibliography hanging-indent="true" line-spacing="2" entry-spacing="1" et-al-min="3" et-al-use-first="1">
        <layout><text variable="title"/></layout>
      </bibliography>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    let b = s.bibliography.as_ref().unwrap();
    assert!(b.hanging_indent);
    assert_eq!(b.line_spacing, Some(2.0));
    assert_eq!(b.entry_spacing, Some(1.0));
    assert_eq!(b.et_al_min, Some(3));
    assert_eq!(b.et_al_use_first, Some(1));
}

// ─────────────────────────────────────────────────────────────────
// Locale terms — plural body
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_locale_term_with_plural_body() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
  <terms>
    <term name="page">
      <single>page</single>
      <multiple>pages</multiple>
    </term>
  </terms>
</locale>"#;
    let l = parse_locale(xml).unwrap();
    let t = l.terms.iter().find(|t| t.name == "page").unwrap();
    assert!(matches!(
        &t.body,
        TermBody::Plural { single, multiple } if single == "page" && multiple == "pages"
    ));
}

#[test]
fn parse_locale_term_with_form_and_gender() {
    let xml = r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
  <terms>
    <term name="month-01" form="short" gender="masculine">Jan.</term>
  </terms>
</locale>"#;
    let l = parse_locale(xml).unwrap();
    let t = &l.terms[0];
    assert_eq!(t.form, TermForm::Short);
    assert_eq!(t.gender, Some(TermGender::Masculine));
    assert!(matches!(&t.body, TermBody::Flat(s) if s == "Jan."));
}

#[test]
fn parse_locale_style_options() {
    let xml = r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
  <style-options punctuation-in-quote="true" limit-day-ordinals-to-day-1="true"/>
</locale>"#;
    let l = parse_locale(xml).unwrap();
    assert!(l.style_options.punctuation_in_quote);
    assert!(l.style_options.limit_day_ordinals_to_day_1);
}

// ─────────────────────────────────────────────────────────────────
// Vertical-align / display / font-variant / text-decoration
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_text_with_vertical_align_sup() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" vertical-align="sup"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert_eq!(t.formatting.vertical_align, Some(VerticalAlign::Sup));
}

#[test]
fn parse_text_with_display_block() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" display="block"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert_eq!(t.display, Some(Display::Block));
}

#[test]
fn parse_text_with_font_variant_small_caps() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-variant="small-caps"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert_eq!(t.formatting.font_variant, Some(FontVariant::SmallCaps));
}

#[test]
fn parse_text_with_text_decoration_underline() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" text-decoration="underline"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!("expected text");
    };
    assert_eq!(
        t.formatting.text_decoration,
        Some(TextDecoration::Underline)
    );
}
