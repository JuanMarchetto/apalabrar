//! Phase 5.2-impl/parser — CSL XML → AST.
//!
//! quick-xml event-based parser. Pragmatic over pretty:
//! - One function per construct.
//! - Unknown attributes are silently ignored (CSL is lenient by spec).
//! - Unknown text-case / font-style / etc. enum values ignored
//!   rather than erroring (forward-compat policy).
//! - Mixed-content elements with text bodies are rare in CSL — most
//!   children are nested elements; text nodes appear only inside
//!   `<term>`s and informational nodes.

use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::ast::*;

// ─────────────────────────────────────────────────────────────────
// Public entry points
// ─────────────────────────────────────────────────────────────────

/// Parse a CSL style XML document. `id` is the lookup id (filename-
/// derived, eg. "apa"); not parsed from the XML.
pub fn parse_style(xml: &str, id: &str) -> Result<Style, ParseError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) if e.local_name().as_ref() == b"style" => {
                return parse_style_inner(&mut reader, &e, id);
            }
            Event::Eof => return Err(ParseError::MissingRoot("style".into())),
            _ => {}
        }
        buf.clear();
    }
}

/// Parse a CSL standalone-locale XML document.
pub fn parse_locale(xml: &str) -> Result<Locale, ParseError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) if e.local_name().as_ref() == b"locale" => {
                return parse_locale_inner(&mut reader, &e);
            }
            Event::Eof => return Err(ParseError::MissingRoot("locale".into())),
            _ => {}
        }
        buf.clear();
    }
}

// ─────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("xml read error: {0}")]
    Xml(String),
    #[error("missing root element <{0}>")]
    MissingRoot(String),
    #[error("missing required attribute @{attr} on <{element}>")]
    MissingAttr { element: String, attr: String },
    #[error("unexpected element <{0}>")]
    UnexpectedElement(String),
}

impl ParseError {
    fn xml(e: quick_xml::Error) -> Self {
        Self::Xml(e.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────
// <style>
// ─────────────────────────────────────────────────────────────────

fn parse_style_inner(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart,
    id: &str,
) -> Result<Style, ParseError> {
    let class = match attr(start, "class").as_deref() {
        Some("note") => StyleClass::Note,
        _ => StyleClass::InText,
    };
    let default_locale = attr(start, "default-locale");
    let mut citation = Citation::default();
    let mut bibliography: Option<Bibliography> = None;
    let mut macros: BTreeMap<String, Vec<Element>> = BTreeMap::new();
    let mut locale_overrides: Vec<Locale> = Vec::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"citation" => citation = parse_citation(reader, &e)?,
                    b"bibliography" => bibliography = Some(parse_bibliography(reader, &e)?),
                    b"macro" => {
                        let macro_name = attr(&e, "name").ok_or(ParseError::MissingAttr {
                            element: "macro".into(),
                            attr: "name".into(),
                        })?;
                        let body = parse_layout_body(reader, b"macro")?;
                        macros.insert(macro_name, body);
                    }
                    b"locale" => locale_overrides.push(parse_locale_inner(reader, &e)?),
                    b"info" => skip_to_end(reader, b"info")?,
                    _ => skip_to_end(reader, &name)?,
                }
            }
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"locale" {
                    // Empty in-style locale (rare, but valid).
                    locale_overrides.push(parse_locale_attrs_only(&e));
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"style" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(Style {
        id: id.to_string(),
        class,
        default_locale,
        citation,
        bibliography,
        macros,
        locale_overrides,
    })
}

// ─────────────────────────────────────────────────────────────────
// <citation> / <bibliography>
// ─────────────────────────────────────────────────────────────────

fn parse_citation(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<Citation, ParseError> {
    let mut c = Citation {
        et_al_min: u32_attr(start, "et-al-min"),
        et_al_use_first: u32_attr(start, "et-al-use-first"),
        et_al_subsequent_min: u32_attr(start, "et-al-subsequent-min"),
        et_al_subsequent_use_first: u32_attr(start, "et-al-subsequent-use-first"),
        et_al_use_last: bool_attr(start, "et-al-use-last"),
        disambiguate_add_givenname: bool_attr(start, "disambiguate-add-givenname"),
        disambiguate_add_names: bool_attr(start, "disambiguate-add-names"),
        disambiguate_add_year_suffix: bool_attr(start, "disambiguate-add-year-suffix"),
        givenname_disambiguation_rule: attr(start, "givenname-disambiguation-rule"),
        collapse: attr(start, "collapse"),
        names_delimiter: attr(start, "names-delimiter"),
        name_delimiter: attr(start, "name-delimiter"),
        initialize_with_hyphen: bool_attr(start, "initialize-with-hyphen"),
        ..Citation::default()
    };
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"layout" => c.layout = parse_layout(reader, &e)?,
                    b"sort" => c.sort = parse_sort(reader)?,
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"citation" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(c)
}

fn parse_bibliography(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart,
) -> Result<Bibliography, ParseError> {
    let mut b = Bibliography {
        hanging_indent: bool_attr(start, "hanging-indent"),
        line_spacing: f32_attr(start, "line-spacing"),
        entry_spacing: f32_attr(start, "entry-spacing"),
        second_field_align: attr(start, "second-field-align"),
        subsequent_author_substitute: attr(start, "subsequent-author-substitute"),
        subsequent_author_substitute_rule: attr(start, "subsequent-author-substitute-rule"),
        et_al_min: u32_attr(start, "et-al-min"),
        et_al_use_first: u32_attr(start, "et-al-use-first"),
        names_delimiter: attr(start, "names-delimiter"),
        name_delimiter: attr(start, "name-delimiter"),
        ..Bibliography::default()
    };
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"layout" => b.layout = parse_layout(reader, &e)?,
                    b"sort" => b.sort = parse_sort(reader)?,
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"bibliography" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(b)
}

// ─────────────────────────────────────────────────────────────────
// <layout> + body
// ─────────────────────────────────────────────────────────────────

fn parse_layout(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<Layout, ParseError> {
    Ok(Layout {
        delimiter: attr(start, "delimiter"),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        formatting: parse_formatting(start),
        body: parse_layout_body(reader, b"layout")?,
    })
}

/// Read elements until the matching end tag of `parent`. Used by
/// layout, group, macro, substitute, if/else-if/else.
fn parse_layout_body(
    reader: &mut Reader<&[u8]>,
    parent: &[u8],
) -> Result<Vec<Element>, ParseError> {
    let mut body: Vec<Element> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"text" => body.push(Element::Text(parse_text(&e))),
                    b"number" => body.push(Element::Number(parse_number(&e))),
                    b"label" => body.push(Element::Label(parse_label(&e))),
                    b"names" => body.push(Element::Names(Box::new(parse_names_from_attrs(&e)))),
                    b"date" => body.push(Element::Date(Box::new(parse_date_from_attrs(&e)))),
                    _ => {}
                }
            }
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"text" => {
                        body.push(Element::Text(parse_text(&e)));
                        skip_to_end(reader, b"text")?;
                    }
                    b"number" => {
                        body.push(Element::Number(parse_number(&e)));
                        skip_to_end(reader, b"number")?;
                    }
                    b"label" => {
                        body.push(Element::Label(parse_label(&e)));
                        skip_to_end(reader, b"label")?;
                    }
                    b"group" => body.push(Element::Group(parse_group(reader, &e)?)),
                    b"choose" => body.push(Element::Choose(parse_choose(reader, &e)?)),
                    b"names" => body.push(Element::Names(Box::new(parse_names(reader, &e)?))),
                    b"date" => body.push(Element::Date(Box::new(parse_date(reader, &e)?))),
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::End(e) if e.local_name().as_ref() == parent => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(body)
}

// ─────────────────────────────────────────────────────────────────
// <text> / <number> / <label>
// ─────────────────────────────────────────────────────────────────

fn parse_text(start: &BytesStart) -> TextElement {
    let source = if let Some(v) = attr(start, "variable") {
        TextSource::Variable(v)
    } else if let Some(m) = attr(start, "macro") {
        TextSource::Macro(m)
    } else if let Some(t) = attr(start, "term") {
        TextSource::Term(t)
    } else {
        TextSource::Value(attr(start, "value").unwrap_or_default())
    };
    TextElement {
        source,
        formatting: parse_formatting(start),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        quotes: bool_attr(start, "quotes"),
        strip_periods: bool_attr(start, "strip-periods"),
        text_case: text_case_from(attr(start, "text-case").as_deref()),
        display: display_from(attr(start, "display").as_deref()),
        form: attr(start, "form"),
        plural: attr(start, "plural"),
    }
}

fn parse_number(start: &BytesStart) -> NumberElement {
    NumberElement {
        variable: attr(start, "variable").unwrap_or_default(),
        form: number_form_from(attr(start, "form").as_deref()),
        formatting: parse_formatting(start),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        text_case: text_case_from(attr(start, "text-case").as_deref()),
        display: display_from(attr(start, "display").as_deref()),
    }
}

fn parse_label(start: &BytesStart) -> LabelElement {
    LabelElement {
        variable: attr(start, "variable"),
        form: attr(start, "form"),
        plural: attr(start, "plural"),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        strip_periods: bool_attr(start, "strip-periods"),
        text_case: text_case_from(attr(start, "text-case").as_deref()),
        formatting: parse_formatting(start),
    }
}

// ─────────────────────────────────────────────────────────────────
// <group>
// ─────────────────────────────────────────────────────────────────

fn parse_group(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<GroupElement, ParseError> {
    Ok(GroupElement {
        delimiter: attr(start, "delimiter"),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        formatting: parse_formatting(start),
        display: display_from(attr(start, "display").as_deref()),
        body: parse_layout_body(reader, b"group")?,
    })
}

// ─────────────────────────────────────────────────────────────────
// <choose>
// ─────────────────────────────────────────────────────────────────

fn parse_choose(
    reader: &mut Reader<&[u8]>,
    _start: &BytesStart,
) -> Result<ChooseElement, ParseError> {
    let mut branches: Vec<ChooseBranch> = Vec::new();
    let mut else_body: Vec<Element> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"if" => {
                        let conditions = parse_conditions(&e);
                        let body = parse_layout_body(reader, b"if")?;
                        branches.push(ChooseBranch { conditions, body });
                    }
                    b"else-if" => {
                        let conditions = parse_conditions(&e);
                        let body = parse_layout_body(reader, b"else-if")?;
                        branches.push(ChooseBranch { conditions, body });
                    }
                    b"else" => {
                        else_body = parse_layout_body(reader, b"else")?;
                    }
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::Empty(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"if" | b"else-if" => {
                        let conditions = parse_conditions(&e);
                        branches.push(ChooseBranch {
                            conditions,
                            body: Vec::new(),
                        });
                    }
                    b"else" => {}
                    _ => {}
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"choose" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(ChooseElement {
        branches,
        else_body,
    })
}

fn parse_conditions(start: &BytesStart) -> Conditions {
    let match_kind = match attr(start, "match").as_deref() {
        Some("any") => MatchKind::Any,
        Some("none") => MatchKind::None,
        _ => MatchKind::All,
    };
    Conditions {
        match_kind,
        variable: split_attr(start, "variable"),
        item_type: split_attr(start, "type"),
        is_numeric: split_attr(start, "is-numeric"),
        is_uncertain_date: split_attr(start, "is-uncertain-date"),
        locator: split_attr(start, "locator"),
        position: split_attr(start, "position"),
        disambiguate: bool_attr(start, "disambiguate"),
    }
}

// ─────────────────────────────────────────────────────────────────
// <names>
// ─────────────────────────────────────────────────────────────────

fn parse_names_from_attrs(start: &BytesStart) -> NamesElement {
    NamesElement {
        variables: split_attr(start, "variable"),
        delimiter: attr(start, "delimiter"),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        formatting: parse_formatting(start),
        display: display_from(attr(start, "display").as_deref()),
        ..NamesElement::default()
    }
}

fn parse_names(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<NamesElement, ParseError> {
    let mut names = parse_names_from_attrs(start);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"name" => names.name = Some(parse_name_from_attrs(&e)),
                    b"et-al" => names.et_al = Some(parse_et_al(&e)),
                    b"label" => names.label = Some(parse_label(&e)),
                    _ => {}
                }
            }
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"name" => names.name = Some(parse_name(reader, &e)?),
                    b"et-al" => {
                        names.et_al = Some(parse_et_al(&e));
                        skip_to_end(reader, b"et-al")?;
                    }
                    b"label" => {
                        names.label = Some(parse_label(&e));
                        skip_to_end(reader, b"label")?;
                    }
                    b"substitute" => {
                        names.substitute = parse_layout_body(reader, b"substitute")?;
                    }
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"names" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(names)
}

fn parse_name_from_attrs(start: &BytesStart) -> NameElement {
    NameElement {
        form: name_form_from(attr(start, "form").as_deref()),
        and: name_and_from(attr(start, "and").as_deref()),
        delimiter: attr(start, "delimiter"),
        delimiter_precedes_et_al: attr(start, "delimiter-precedes-et-al"),
        delimiter_precedes_last: attr(start, "delimiter-precedes-last"),
        et_al_min: u32_attr(start, "et-al-min"),
        et_al_use_first: u32_attr(start, "et-al-use-first"),
        et_al_use_last: bool_attr(start, "et-al-use-last"),
        initialize: !matches!(attr(start, "initialize").as_deref(), Some("false")),
        initialize_with: attr(start, "initialize-with"),
        initialize_with_hyphen: bool_attr(start, "initialize-with-hyphen"),
        name_as_sort_order: name_as_sort_order_from(attr(start, "name-as-sort-order").as_deref()),
        sort_separator: attr(start, "sort-separator"),
        formatting: parse_formatting(start),
        ..NameElement::default()
    }
}

fn parse_name(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<NameElement, ParseError> {
    let mut name = parse_name_from_attrs(start);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"name-part" {
                    if let Some(part) = parse_name_part(&e) {
                        name.name_parts.push(part);
                    }
                }
            }
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                if n == b"name-part" {
                    if let Some(part) = parse_name_part(&e) {
                        name.name_parts.push(part);
                    }
                    skip_to_end(reader, b"name-part")?;
                } else {
                    skip_to_end(reader, &n)?;
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"name" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(name)
}

fn parse_name_part(start: &BytesStart) -> Option<NamePart> {
    Some(NamePart {
        name: attr(start, "name")?,
        formatting: parse_formatting(start),
        text_case: text_case_from(attr(start, "text-case").as_deref()),
    })
}

fn parse_et_al(start: &BytesStart) -> EtAlElement {
    EtAlElement {
        term: attr(start, "term"),
        formatting: parse_formatting(start),
    }
}

// ─────────────────────────────────────────────────────────────────
// <date>
// ─────────────────────────────────────────────────────────────────

fn parse_date_from_attrs(start: &BytesStart) -> DateElement {
    DateElement {
        variable: attr(start, "variable").unwrap_or_default(),
        form: date_form_from(attr(start, "form").as_deref()),
        date_parts_filter: date_parts_filter_from(attr(start, "date-parts").as_deref()),
        date_parts: Vec::new(),
        delimiter: attr(start, "delimiter"),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        formatting: parse_formatting(start),
        display: display_from(attr(start, "display").as_deref()),
    }
}

fn parse_date(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<DateElement, ParseError> {
    let mut d = parse_date_from_attrs(start);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"date-part"
                    && let Some(p) = parse_date_part(&e)
                {
                    d.date_parts.push(p);
                }
            }
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                if n == b"date-part" {
                    if let Some(p) = parse_date_part(&e) {
                        d.date_parts.push(p);
                    }
                    skip_to_end(reader, b"date-part")?;
                } else {
                    skip_to_end(reader, &n)?;
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"date" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(d)
}

fn parse_date_part(start: &BytesStart) -> Option<DatePart> {
    let name = match attr(start, "name").as_deref() {
        Some("year") => DatePartName::Year,
        Some("month") => DatePartName::Month,
        Some("day") => DatePartName::Day,
        _ => return None,
    };
    Some(DatePart {
        name,
        form: date_part_form_from(name, attr(start, "form").as_deref()),
        prefix: attr(start, "prefix"),
        suffix: attr(start, "suffix"),
        range_delimiter: attr(start, "range-delimiter"),
        formatting: parse_formatting(start),
        text_case: text_case_from(attr(start, "text-case").as_deref()),
        strip_periods: bool_attr(start, "strip-periods"),
    })
}

// ─────────────────────────────────────────────────────────────────
// <sort>
// ─────────────────────────────────────────────────────────────────

fn parse_sort(reader: &mut Reader<&[u8]>) -> Result<Vec<SortKey>, ParseError> {
    let mut keys = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"key"
                    && let Some(k) = parse_sort_key(&e)
                {
                    keys.push(k);
                }
            }
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                if n == b"key" {
                    if let Some(k) = parse_sort_key(&e) {
                        keys.push(k);
                    }
                    skip_to_end(reader, b"key")?;
                } else {
                    skip_to_end(reader, &n)?;
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"sort" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(keys)
}

fn parse_sort_key(start: &BytesStart) -> Option<SortKey> {
    let source = if let Some(v) = attr(start, "variable") {
        SortSource::Variable(v)
    } else if let Some(m) = attr(start, "macro") {
        SortSource::Macro(m)
    } else {
        return None;
    };
    Some(SortKey {
        source,
        sort_descending: matches!(attr(start, "sort").as_deref(), Some("descending")),
        names_min: u32_attr(start, "names-min"),
        names_use_first: u32_attr(start, "names-use-first"),
        names_use_last: bool_attr(start, "names-use-last"),
    })
}

// ─────────────────────────────────────────────────────────────────
// <locale>
// ─────────────────────────────────────────────────────────────────

fn parse_locale_inner(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart,
) -> Result<Locale, ParseError> {
    let mut loc = parse_locale_attrs_only(start);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"style-options" {
                    loc.style_options = parse_style_options(&e);
                }
            }
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"style-options" => {
                        loc.style_options = parse_style_options(&e);
                        skip_to_end(reader, b"style-options")?;
                    }
                    b"terms" => loc.terms = parse_terms(reader)?,
                    b"date" => match attr(&e, "form").as_deref() {
                        Some("text") => loc.date_text = Some(parse_date(reader, &e)?),
                        Some("numeric") => loc.date_numeric = Some(parse_date(reader, &e)?),
                        _ => skip_to_end(reader, b"date")?,
                    },
                    b"info" => skip_to_end(reader, b"info")?,
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"locale" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(loc)
}

fn parse_locale_attrs_only(start: &BytesStart) -> Locale {
    Locale {
        lang: attr_xml_lang(start),
        ..Locale::default()
    }
}

fn parse_style_options(start: &BytesStart) -> StyleOptions {
    StyleOptions {
        punctuation_in_quote: bool_attr(start, "punctuation-in-quote"),
        limit_day_ordinals_to_day_1: bool_attr(start, "limit-day-ordinals-to-day-1"),
    }
}

fn parse_terms(reader: &mut Reader<&[u8]>) -> Result<Vec<Term>, ParseError> {
    let mut terms = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"term" {
                    terms.push(Term {
                        name: attr(&e, "name").unwrap_or_default(),
                        form: term_form_from(attr(&e, "form").as_deref()),
                        gender: term_gender_from(attr(&e, "gender").as_deref()),
                        gender_form: term_gender_from(attr(&e, "gender-form").as_deref()),
                        match_attr: attr(&e, "match"),
                        body: TermBody::Flat(String::new()),
                    });
                }
            }
            Event::Start(e) => {
                if e.local_name().as_ref() == b"term" {
                    let attrs = (
                        attr(&e, "name").unwrap_or_default(),
                        term_form_from(attr(&e, "form").as_deref()),
                        term_gender_from(attr(&e, "gender").as_deref()),
                        term_gender_from(attr(&e, "gender-form").as_deref()),
                        attr(&e, "match"),
                    );
                    let body = parse_term_body(reader)?;
                    terms.push(Term {
                        name: attrs.0,
                        form: attrs.1,
                        gender: attrs.2,
                        gender_form: attrs.3,
                        match_attr: attrs.4,
                        body,
                    });
                } else {
                    let n = e.local_name().as_ref().to_vec();
                    skip_to_end(reader, &n)?;
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"terms" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(terms)
}

fn parse_term_body(reader: &mut Reader<&[u8]>) -> Result<TermBody, ParseError> {
    let mut single: Option<String> = None;
    let mut multiple: Option<String> = None;
    let mut flat: Option<String> = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) => {
                let n = e.local_name().as_ref().to_vec();
                match n.as_slice() {
                    b"single" => single = Some(read_text(reader, b"single")?),
                    b"multiple" => multiple = Some(read_text(reader, b"multiple")?),
                    _ => skip_to_end(reader, &n)?,
                }
            }
            Event::Text(t) => {
                let s = t.unescape().map_err(ParseError::xml)?.into_owned();
                if !s.trim().is_empty() {
                    flat = Some(match flat {
                        Some(prev) => prev + &s,
                        None => s,
                    });
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"term" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(match (single, multiple) {
        (Some(s), Some(m)) => TermBody::Plural {
            single: s,
            multiple: m,
        },
        _ => TermBody::Flat(flat.unwrap_or_default()),
    })
}

fn read_text(reader: &mut Reader<&[u8]>, parent: &[u8]) -> Result<String, ParseError> {
    let mut out = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Text(t) => {
                let s = t.unescape().map_err(ParseError::xml)?;
                out.push_str(&s);
            }
            Event::End(e) if e.local_name().as_ref() == parent => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────
// Common attribute helpers
// ─────────────────────────────────────────────────────────────────

fn parse_formatting(start: &BytesStart) -> Formatting {
    Formatting {
        font_style: font_style_from(attr(start, "font-style").as_deref()),
        font_weight: font_weight_from(attr(start, "font-weight").as_deref()),
        font_variant: font_variant_from(attr(start, "font-variant").as_deref()),
        text_decoration: text_decoration_from(attr(start, "text-decoration").as_deref()),
        vertical_align: vertical_align_from(attr(start, "vertical-align").as_deref()),
    }
}

fn attr(start: &BytesStart, name: &str) -> Option<String> {
    for a in start.attributes().with_checks(false).flatten() {
        if a.key.local_name().as_ref() == name.as_bytes() {
            return Some(decode_attr_value(&a));
        }
    }
    None
}

/// xml:lang requires checking the prefixed form.
fn attr_xml_lang(start: &BytesStart) -> Option<String> {
    for a in start.attributes().with_checks(false).flatten() {
        if a.key.as_ref() == b"xml:lang" {
            return Some(decode_attr_value(&a));
        }
    }
    None
}

/// Best-effort decode of an attribute value to UTF-8 String.
/// Falls back to lossy decoding when the attribute value contains
/// invalid UTF-8 (very rare in practice for CSL XML).
fn decode_attr_value(a: &quick_xml::events::attributes::Attribute) -> String {
    match std::str::from_utf8(&a.value) {
        Ok(s) => unescape_xml(s),
        Err(_) => unescape_xml(&String::from_utf8_lossy(&a.value)),
    }
}

/// Minimal XML entity unescape: covers the 5 standard entities.
/// CSL XML rarely uses character references; full entity support
/// (including custom DTDs) is overkill for our bundled assets.
fn unescape_xml(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn bool_attr(start: &BytesStart, name: &str) -> bool {
    matches!(attr(start, name).as_deref(), Some("true"))
}

fn u32_attr(start: &BytesStart, name: &str) -> Option<u32> {
    attr(start, name).and_then(|s| s.parse().ok())
}

fn f32_attr(start: &BytesStart, name: &str) -> Option<f32> {
    attr(start, name).and_then(|s| s.parse().ok())
}

fn split_attr(start: &BytesStart, name: &str) -> Vec<String> {
    attr(start, name)
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default()
}

fn skip_to_end(reader: &mut Reader<&[u8]>, name: &[u8]) -> Result<(), ParseError> {
    let mut depth = 1;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(ParseError::xml)? {
            Event::Start(e) if e.local_name().as_ref() == name => depth += 1,
            Event::End(e) if e.local_name().as_ref() == name => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
        buf.clear();
    }
}

// ─────────────────────────────────────────────────────────────────
// Enum value parsers
// ─────────────────────────────────────────────────────────────────

fn font_style_from(s: Option<&str>) -> Option<FontStyle> {
    match s? {
        "normal" => Some(FontStyle::Normal),
        "italic" => Some(FontStyle::Italic),
        "oblique" => Some(FontStyle::Oblique),
        _ => None,
    }
}

fn font_weight_from(s: Option<&str>) -> Option<FontWeight> {
    match s? {
        "normal" => Some(FontWeight::Normal),
        "bold" => Some(FontWeight::Bold),
        "light" => Some(FontWeight::Light),
        _ => None,
    }
}

fn font_variant_from(s: Option<&str>) -> Option<FontVariant> {
    match s? {
        "normal" => Some(FontVariant::Normal),
        "small-caps" => Some(FontVariant::SmallCaps),
        _ => None,
    }
}

fn text_decoration_from(s: Option<&str>) -> Option<TextDecoration> {
    match s? {
        "none" => Some(TextDecoration::None),
        "underline" => Some(TextDecoration::Underline),
        _ => None,
    }
}

fn vertical_align_from(s: Option<&str>) -> Option<VerticalAlign> {
    match s? {
        "baseline" => Some(VerticalAlign::Baseline),
        "sup" => Some(VerticalAlign::Sup),
        "sub" => Some(VerticalAlign::Sub),
        _ => None,
    }
}

fn text_case_from(s: Option<&str>) -> Option<TextCase> {
    match s? {
        "lowercase" => Some(TextCase::Lowercase),
        "uppercase" => Some(TextCase::Uppercase),
        "capitalize-all" => Some(TextCase::Capitalize),
        "capitalize-first" => Some(TextCase::CapitalizeFirst),
        "sentence" => Some(TextCase::Sentence),
        "title" => Some(TextCase::Title),
        _ => None,
    }
}

fn display_from(s: Option<&str>) -> Option<Display> {
    match s? {
        "block" => Some(Display::Block),
        "left-margin" => Some(Display::LeftMargin),
        "right-inline" => Some(Display::RightInline),
        "indent" => Some(Display::Indent),
        _ => None,
    }
}

fn number_form_from(s: Option<&str>) -> Option<NumberForm> {
    match s? {
        "numeric" => Some(NumberForm::Numeric),
        "ordinal" => Some(NumberForm::Ordinal),
        "long-ordinal" => Some(NumberForm::LongOrdinal),
        "roman" => Some(NumberForm::Roman),
        _ => None,
    }
}

fn name_form_from(s: Option<&str>) -> Option<NameForm> {
    match s? {
        "long" => Some(NameForm::Long),
        "short" => Some(NameForm::Short),
        "count" => Some(NameForm::Count),
        _ => None,
    }
}

fn name_and_from(s: Option<&str>) -> Option<NameAnd> {
    match s? {
        "text" => Some(NameAnd::Text),
        "symbol" => Some(NameAnd::Symbol),
        _ => None,
    }
}

fn name_as_sort_order_from(s: Option<&str>) -> Option<NameAsSortOrder> {
    match s? {
        "first" => Some(NameAsSortOrder::First),
        "all" => Some(NameAsSortOrder::All),
        _ => None,
    }
}

fn date_form_from(s: Option<&str>) -> Option<DateForm> {
    match s? {
        "text" => Some(DateForm::Text),
        "numeric" => Some(DateForm::Numeric),
        _ => None,
    }
}

fn date_parts_filter_from(s: Option<&str>) -> Option<DatePartsFilter> {
    match s? {
        "year" => Some(DatePartsFilter::Year),
        "year-month" => Some(DatePartsFilter::YearMonth),
        "year-month-day" => Some(DatePartsFilter::YearMonthDay),
        _ => None,
    }
}

fn date_part_form_from(name: DatePartName, s: Option<&str>) -> Option<DatePartForm> {
    let s = s?;
    match (name, s) {
        (_, "numeric") => Some(DatePartForm::Numeric),
        (_, "numeric-leading-zeros") => Some(DatePartForm::NumericLeadingZeros),
        (_, "ordinal") => Some(DatePartForm::Ordinal),
        (DatePartName::Month, "long") => Some(DatePartForm::Long),
        (DatePartName::Month, "short") => Some(DatePartForm::Short),
        (DatePartName::Year, "short") => Some(DatePartForm::Short2),
        _ => None,
    }
}

fn term_form_from(s: Option<&str>) -> TermForm {
    match s {
        Some("short") => TermForm::Short,
        Some("verb") => TermForm::Verb,
        Some("verb-short") => TermForm::VerbShort,
        Some("symbol") => TermForm::Symbol,
        _ => TermForm::Long,
    }
}

fn term_gender_from(s: Option<&str>) -> Option<TermGender> {
    match s? {
        "masculine" => Some(TermGender::Masculine),
        "feminine" => Some(TermGender::Feminine),
        "neuter" => Some(TermGender::Neuter),
        _ => None,
    }
}
