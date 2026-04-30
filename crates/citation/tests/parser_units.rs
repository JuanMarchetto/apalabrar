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

// ─────────────────────────────────────────────────────────────────
// Coverage bump (Phase 5.2-impl/renderer-3): targeted branch tests
// ─────────────────────────────────────────────────────────────────

#[test]
fn parse_text_with_font_style_oblique() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-style="oblique"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(t.formatting.font_style, Some(FontStyle::Oblique));
}

#[test]
fn parse_text_with_font_style_normal() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-style="normal"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(t.formatting.font_style, Some(FontStyle::Normal));
}

#[test]
fn parse_text_with_font_weight_light() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-weight="light"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(t.formatting.font_weight, Some(FontWeight::Light));
}

#[test]
fn parse_text_with_font_weight_normal() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-weight="normal"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(t.formatting.font_weight, Some(FontWeight::Normal));
}

#[test]
fn parse_text_with_font_variant_normal() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" font-variant="normal"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(t.formatting.font_variant, Some(FontVariant::Normal));
}

#[test]
fn parse_text_with_text_decoration_none() {
    let s = parse_style(
        &wrap_style(r#"<text variable="title" text-decoration="none"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(t.formatting.text_decoration, Some(TextDecoration::None));
}

#[test]
fn parse_text_with_vertical_align_baseline_and_sub() {
    for (val, expected) in [
        ("baseline", VerticalAlign::Baseline),
        ("sub", VerticalAlign::Sub),
    ] {
        let s = parse_style(
            &wrap_style(&format!(
                r#"<text variable="title" vertical-align="{val}"/>"#
            )),
            "x",
        )
        .unwrap();
        let Element::Text(t) = first_layout_elem(&s) else {
            panic!()
        };
        assert_eq!(t.formatting.vertical_align, Some(expected));
    }
}

#[test]
fn parse_text_with_each_text_case_variant() {
    for (val, expected) in [
        ("lowercase", TextCase::Lowercase),
        ("uppercase", TextCase::Uppercase),
        ("capitalize-first", TextCase::CapitalizeFirst),
        ("sentence", TextCase::Sentence),
        ("title", TextCase::Title),
    ] {
        let s = parse_style(
            &wrap_style(&format!(r#"<text variable="title" text-case="{val}"/>"#)),
            "x",
        )
        .unwrap();
        let Element::Text(t) = first_layout_elem(&s) else {
            panic!()
        };
        assert_eq!(t.text_case, Some(expected));
    }
}

#[test]
fn parse_text_with_each_display_variant() {
    for (val, expected) in [
        ("left-margin", Display::LeftMargin),
        ("right-inline", Display::RightInline),
        ("indent", Display::Indent),
    ] {
        let s = parse_style(
            &wrap_style(&format!(r#"<text variable="title" display="{val}"/>"#)),
            "x",
        )
        .unwrap();
        let Element::Text(t) = first_layout_elem(&s) else {
            panic!()
        };
        assert_eq!(t.display, Some(expected));
    }
}

#[test]
fn parse_name_with_form_count() {
    let s = parse_style(
        &wrap_style(r#"<names variable="author"><name form="count"/></names>"#),
        "x",
    )
    .unwrap();
    let Element::Names(n) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(n.name.as_ref().unwrap().form, Some(NameForm::Count));
}

#[test]
fn parse_name_as_sort_order_all() {
    let s = parse_style(
        &wrap_style(r#"<names variable="author"><name name-as-sort-order="all"/></names>"#),
        "x",
    )
    .unwrap();
    let Element::Names(n) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(
        n.name.as_ref().unwrap().name_as_sort_order,
        Some(NameAsSortOrder::All)
    );
}

#[test]
fn parse_date_with_form_numeric() {
    let s = parse_style(
        &wrap_style(r#"<date variable="issued" form="numeric"/>"#),
        "x",
    )
    .unwrap();
    let Element::Date(d) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(d.form, Some(DateForm::Numeric));
}

#[test]
fn parse_date_part_with_each_form_variant() {
    let s = parse_style(
        &wrap_style(
            r#"<date variable="issued">
                <date-part name="year" form="short"/>
                <date-part name="month" form="long"/>
                <date-part name="day" form="ordinal"/>
            </date>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Date(d) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(d.date_parts[0].form, Some(DatePartForm::Short2));
    assert_eq!(d.date_parts[1].form, Some(DatePartForm::Long));
    assert_eq!(d.date_parts[2].form, Some(DatePartForm::Ordinal));
}

#[test]
fn parse_date_parts_filter_all_variants() {
    for (val, expected) in [
        ("year", DatePartsFilter::Year),
        ("year-month-day", DatePartsFilter::YearMonthDay),
    ] {
        let s = parse_style(
            &wrap_style(&format!(
                r#"<date variable="issued" form="text" date-parts="{val}"/>"#
            )),
            "x",
        )
        .unwrap();
        let Element::Date(d) = first_layout_elem(&s) else {
            panic!()
        };
        assert_eq!(d.date_parts_filter, Some(expected));
    }
}

#[test]
fn parse_term_with_each_form_variant() {
    for (val, expected) in [
        ("verb", TermForm::Verb),
        ("verb-short", TermForm::VerbShort),
        ("symbol", TermForm::Symbol),
    ] {
        let xml = format!(
            r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
              <terms><term name="and" form="{val}">x</term></terms>
            </locale>"#
        );
        let l = parse_locale(&xml).unwrap();
        assert_eq!(l.terms[0].form, expected);
    }
}

#[test]
fn parse_term_with_each_gender_variant() {
    for (val, expected) in [
        ("feminine", TermGender::Feminine),
        ("neuter", TermGender::Neuter),
    ] {
        let xml = format!(
            r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
              <terms><term name="x" gender="{val}">y</term></terms>
            </locale>"#
        );
        let l = parse_locale(&xml).unwrap();
        assert_eq!(l.terms[0].gender, Some(expected));
    }
}

#[test]
fn parse_choose_with_locator_and_position_conditions() {
    let s = parse_style(
        &wrap_style(
            r#"<choose>
              <if locator="page" position="first" disambiguate="true">
                <text value="x"/>
              </if>
            </choose>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Choose(c) = first_layout_elem(&s) else {
        panic!()
    };
    let cond = &c.branches[0].conditions;
    assert_eq!(cond.locator, vec!["page".to_string()]);
    assert_eq!(cond.position, vec!["first".to_string()]);
    assert!(cond.disambiguate);
}

#[test]
fn parse_choose_with_is_numeric_and_is_uncertain_date_conditions() {
    let s = parse_style(
        &wrap_style(
            r#"<choose>
              <if is-numeric="volume issue" is-uncertain-date="issued">
                <text value="x"/>
              </if>
            </choose>"#,
        ),
        "x",
    )
    .unwrap();
    let Element::Choose(c) = first_layout_elem(&s) else {
        panic!()
    };
    let cond = &c.branches[0].conditions;
    assert_eq!(cond.is_numeric.len(), 2);
    assert_eq!(cond.is_uncertain_date, vec!["issued".to_string()]);
}

#[test]
fn parse_label_with_each_text_case_and_strip_periods() {
    let s = parse_style(
        &wrap_style(r#"<label variable="page" strip-periods="true" text-case="uppercase"/>"#),
        "x",
    )
    .unwrap();
    let Element::Label(l) = first_layout_elem(&s) else {
        panic!()
    };
    assert!(l.strip_periods);
    assert_eq!(l.text_case, Some(TextCase::Uppercase));
}

#[test]
fn parse_locale_with_inline_date_definitions() {
    let xml = r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
      <date form="text">
        <date-part name="month" suffix=" "/>
        <date-part name="day" suffix=", "/>
        <date-part name="year"/>
      </date>
      <date form="numeric">
        <date-part name="year" form="numeric"/>
      </date>
    </locale>"#;
    let l = parse_locale(xml).unwrap();
    assert!(l.date_text.is_some());
    assert!(l.date_numeric.is_some());
    let text = l.date_text.unwrap();
    assert_eq!(text.date_parts.len(), 3);
}

#[test]
fn parse_style_with_full_root_attrs() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl"
        class="note" version="1.0"
        default-locale="es-ES"
        initialize-with=". "
        names-delimiter=", "
        demote-non-dropping-particle="never"
        page-range-format="expanded"></style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.class, StyleClass::Note);
    assert_eq!(s.default_locale.as_deref(), Some("es-ES"));
    assert_eq!(s.initialize_with.as_deref(), Some(". "));
    assert_eq!(s.names_delimiter.as_deref(), Some(", "));
    assert_eq!(s.demote_non_dropping_particle.as_deref(), Some("never"));
    assert_eq!(s.page_range_format.as_deref(), Some("expanded"));
}

#[test]
fn parse_bibliography_with_et_al_use_last() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <citation><layout/></citation>
      <bibliography et-al-use-last="true"><layout/></bibliography>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert!(s.bibliography.unwrap().et_al_use_last);
}

#[test]
fn parse_skip_to_end_handles_nested_elements() {
    // Unknown element with nested children is fully skipped without
    // bleeding into the next sibling — verified by the parent
    // continuing to find <citation>.
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <unknown-element>
        <unknown-element>
          <text value="ignored"/>
        </unknown-element>
      </unknown-element>
      <citation><layout><text value="x"/></layout></citation>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.citation.layout.body.len(), 1);
}

#[test]
fn parse_text_with_macro_source_and_strip_periods() {
    // Strip-periods on a macro reference applies to the expanded
    // macro output (renderer-side; parser just records the attr).
    let s = parse_style(
        &wrap_style(r#"<text macro="author" strip-periods="true"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    assert!(t.strip_periods);
    assert!(matches!(&t.source, TextSource::Macro(m) if m == "author"));
}

#[test]
fn parse_sort_with_names_min_use_first_use_last() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <citation>
        <sort>
          <key macro="author-sort" names-min="3" names-use-first="1" names-use-last="true"/>
        </sort>
        <layout/>
      </citation>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    let k = &s.citation.sort[0];
    assert_eq!(k.names_min, Some(3));
    assert_eq!(k.names_use_first, Some(1));
    assert!(k.names_use_last);
}

#[test]
fn parse_layout_with_prefix_and_suffix() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <citation>
        <layout prefix="(" suffix=")" delimiter="; ">
          <text variable="title"/>
        </layout>
      </citation>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.citation.layout.prefix.as_deref(), Some("("));
    assert_eq!(s.citation.layout.suffix.as_deref(), Some(")"));
    assert_eq!(s.citation.layout.delimiter.as_deref(), Some("; "));
}

#[test]
fn parse_citation_with_full_et_al_attrs() {
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <citation et-al-min="3" et-al-use-first="1"
                et-al-subsequent-min="2" et-al-subsequent-use-first="1"
                et-al-use-last="true"
                disambiguate-add-givenname="true"
                disambiguate-add-names="true"
                disambiguate-add-year-suffix="true"
                givenname-disambiguation-rule="primary-name-with-initials"
                collapse="year-suffix-ranged"
                names-delimiter=", "
                name-delimiter="; "
                initialize-with-hyphen="true">
        <layout/>
      </citation>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    let c = &s.citation;
    assert_eq!(c.et_al_min, Some(3));
    assert_eq!(c.et_al_use_first, Some(1));
    assert_eq!(c.et_al_subsequent_min, Some(2));
    assert_eq!(c.et_al_subsequent_use_first, Some(1));
    assert!(c.et_al_use_last);
    assert!(c.disambiguate_add_givenname);
    assert!(c.disambiguate_add_names);
    assert!(c.disambiguate_add_year_suffix);
    assert_eq!(
        c.givenname_disambiguation_rule.as_deref(),
        Some("primary-name-with-initials")
    );
    assert_eq!(c.collapse.as_deref(), Some("year-suffix-ranged"));
    assert!(c.initialize_with_hyphen);
}

#[test]
fn parse_style_handles_in_style_locale_with_no_lang() {
    // <locale> inside <style> may omit xml:lang to apply to all
    // locales (rare but valid).
    let xml = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
      <locale>
        <terms><term name="x">y</term></terms>
      </locale>
      <citation><layout/></citation>
    </style>"#;
    let s = parse_style(xml, "x").unwrap();
    assert_eq!(s.locale_overrides.len(), 1);
    assert!(s.locale_overrides[0].lang.is_none());
}

#[test]
fn parse_handles_xml_entities_in_attribute_values() {
    // Standard XML entities in attr values must round-trip correctly.
    let s = parse_style(
        &wrap_style(r#"<text value="a &amp; b &lt; c &gt; d"/>"#),
        "x",
    )
    .unwrap();
    let Element::Text(t) = first_layout_elem(&s) else {
        panic!()
    };
    let TextSource::Value(v) = &t.source else {
        panic!()
    };
    assert_eq!(v, "a & b < c > d");
}

#[test]
fn parse_empty_locale_terms_block() {
    let xml = r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
      <terms></terms>
    </locale>"#;
    let l = parse_locale(xml).unwrap();
    assert!(l.terms.is_empty());
}

#[test]
fn parse_self_closing_locale_terms_block() {
    let xml = r#"<locale xmlns="http://purl.org/net/xbiblio/csl" version="1.0" xml:lang="en-US">
      <terms/>
    </locale>"#;
    let l = parse_locale(xml).unwrap();
    assert!(l.terms.is_empty());
}

#[test]
fn parse_self_closing_names_with_only_attrs() {
    // <names variable="author"/> self-closing is valid CSL —
    // inherits all defaults.
    let s = parse_style(&wrap_style(r#"<names variable="author editor"/>"#), "x").unwrap();
    let Element::Names(n) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(n.variables, vec!["author", "editor"]);
    assert!(n.name.is_none());
}

#[test]
fn parse_self_closing_date_inside_layout() {
    let s = parse_style(&wrap_style(r#"<date variable="issued" form="text"/>"#), "x").unwrap();
    let Element::Date(d) = first_layout_elem(&s) else {
        panic!()
    };
    assert_eq!(d.variable, "issued");
    assert_eq!(d.form, Some(DateForm::Text));
}
