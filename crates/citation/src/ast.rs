//! Phase 5.2 — internal CSL 1.0.2 AST.
//!
//! Mirrors the CSL XML structure closely. Mostly `String`s and
//! `Vec`s; no Cow / lifetime tricks — the parser owns each Style
//! after parsing, and the AST lives behind `Arc` in the cache so
//! cloning is cheap.
//!
//! Coverage scope: every element + attribute that appears in the 5
//! bundled styles (apa, ieee, mla, ama, chicago-notes-bibliography)
//! plus every locale construct. Constructs unique to less-common
//! styles (eg. CSL-M extensions, `<conditions>` group syntax) are
//! deferred until they appear.

use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────
// Style root
// ─────────────────────────────────────────────────────────────────

/// A parsed CSL style. The `id` is the filename-derived identifier
/// (eg. "apa") — NOT the `<info><id>` URL inside the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub id: String,
    pub class: StyleClass,
    pub default_locale: Option<String>,
    /// `<style initialize-with="…">` — global default for `<name>`
    /// elements that don't set their own. APA / MLA put it here.
    pub initialize_with: Option<String>,
    /// `<style names-delimiter="…">`.
    pub names_delimiter: Option<String>,
    /// `<style demote-non-dropping-particle="…">`.
    pub demote_non_dropping_particle: Option<String>,
    /// `<style page-range-format="…">`.
    pub page_range_format: Option<String>,
    pub citation: Citation,
    pub bibliography: Option<Bibliography>,
    pub macros: BTreeMap<String, Vec<Element>>,
    /// In-style locale overrides — `<locale xml:lang="…">` children
    /// of `<style>`. Override the corresponding terms / dates from
    /// the standalone locale file.
    pub locale_overrides: Vec<Locale>,
}

/// CSL style class — controls overall layout (in-text vs footnote).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StyleClass {
    /// Author-date / numeric in-text citation. `<layout>` produces
    /// the in-text form. (apa, ieee, mla, ama)
    #[default]
    InText,
    /// Footnote / endnote citation. `<layout>` produces the full
    /// footnote text. (chicago-notes-bibliography)
    Note,
}

// ─────────────────────────────────────────────────────────────────
// Citation + Bibliography
// ─────────────────────────────────────────────────────────────────

/// `<citation>` element — controls the in-text / inline form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Citation {
    pub layout: Layout,
    pub sort: Vec<SortKey>,
    pub et_al_min: Option<u32>,
    pub et_al_use_first: Option<u32>,
    pub et_al_subsequent_min: Option<u32>,
    pub et_al_subsequent_use_first: Option<u32>,
    pub et_al_use_last: bool,
    pub disambiguate_add_givenname: bool,
    pub disambiguate_add_names: bool,
    pub disambiguate_add_year_suffix: bool,
    pub givenname_disambiguation_rule: Option<String>,
    pub collapse: Option<String>,
    pub names_delimiter: Option<String>,
    pub name_delimiter: Option<String>,
    pub initialize_with_hyphen: bool,
}

/// `<bibliography>` element — controls the bibliography list form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bibliography {
    pub layout: Layout,
    pub sort: Vec<SortKey>,
    pub hanging_indent: bool,
    pub line_spacing: Option<f32>,
    pub entry_spacing: Option<f32>,
    pub second_field_align: Option<String>,
    pub subsequent_author_substitute: Option<String>,
    pub subsequent_author_substitute_rule: Option<String>,
    pub et_al_min: Option<u32>,
    pub et_al_use_first: Option<u32>,
    pub et_al_use_last: bool,
    pub names_delimiter: Option<String>,
    pub name_delimiter: Option<String>,
}

// ─────────────────────────────────────────────────────────────────
// Layout + sort keys
// ─────────────────────────────────────────────────────────────────

/// `<layout>` element — body of a citation/bibliography. Wraps a
/// vec of [`Element`]s with optional delimiter / prefix / suffix
/// and inherited formatting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub delimiter: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub formatting: Formatting,
    pub body: Vec<Element>,
}

/// One sort key inside `<sort>`. Either a variable or a macro
/// reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortKey {
    pub source: SortSource,
    pub sort_descending: bool,
    pub names_min: Option<u32>,
    pub names_use_first: Option<u32>,
    pub names_use_last: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortSource {
    Variable(String),
    Macro(String),
}

// ─────────────────────────────────────────────────────────────────
// Element — inside a layout / group / substitute / if branch
// ─────────────────────────────────────────────────────────────────

/// One emit-able piece of citation output. `Names` and `Date` are
/// boxed because they're significantly larger than the other variants;
/// boxing keeps `Element` compact for vec storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Text(TextElement),
    Number(NumberElement),
    Group(GroupElement),
    Choose(ChooseElement),
    Names(Box<NamesElement>),
    Date(Box<DateElement>),
    Label(LabelElement),
}

// ─────────────────────────────────────────────────────────────────
// Formatting (shared across elements)
// ─────────────────────────────────────────────────────────────────

/// Shared formatting attributes. CSL allows these on most elements.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Formatting {
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_variant: Option<FontVariant>,
    pub text_decoration: Option<TextDecoration>,
    pub vertical_align: Option<VerticalAlign>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDecoration {
    None,
    Underline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlign {
    Baseline,
    Sup,
    Sub,
}

// Common affix / case / quoting attrs. Kept separate from
// `Formatting` because layout / group don't carry text-case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextCase {
    Lowercase,
    Uppercase,
    Capitalize,
    CapitalizeFirst,
    Sentence,
    Title,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Block,
    LeftMargin,
    RightInline,
    Indent,
}

// ─────────────────────────────────────────────────────────────────
// Text element
// ─────────────────────────────────────────────────────────────────

/// `<text>` — emit a variable, term, value, or expanded macro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextElement {
    pub source: TextSource,
    pub formatting: Formatting,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub quotes: bool,
    pub strip_periods: bool,
    pub text_case: Option<TextCase>,
    pub display: Option<Display>,
    /// `form="long"` / `"short"` for terms.
    pub form: Option<String>,
    /// `plural` attr for terms (singular / multiple / always /
    /// contextual).
    pub plural: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextSource {
    /// `variable="container-title"` etc.
    Variable(String),
    /// `macro="author"` — expand by lookup in `Style.macros`.
    Macro(String),
    /// `term="and"` — render the locale term.
    Term(String),
    /// `value="literal text"`.
    Value(String),
}

// ─────────────────────────────────────────────────────────────────
// Number element
// ─────────────────────────────────────────────────────────────────

/// `<number>` — render a numeric variable with optional formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberElement {
    pub variable: String,
    pub form: Option<NumberForm>,
    pub formatting: Formatting,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub text_case: Option<TextCase>,
    pub display: Option<Display>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberForm {
    Numeric,
    Ordinal,
    LongOrdinal,
    Roman,
}

// ─────────────────────────────────────────────────────────────────
// Group element
// ─────────────────────────────────────────────────────────────────

/// `<group>` — wraps children with a delimiter; suppressed entirely
/// if no child renders (CSL "fail group" semantics).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupElement {
    pub delimiter: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub formatting: Formatting,
    pub display: Option<Display>,
    pub body: Vec<Element>,
}

// ─────────────────────────────────────────────────────────────────
// Choose / If
// ─────────────────────────────────────────────────────────────────

/// `<choose>` — branching. First `if`/`else-if` whose conditions
/// match wins; `else` is the fallback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChooseElement {
    pub branches: Vec<ChooseBranch>,
    /// `<else>` body; may be empty.
    pub else_body: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChooseBranch {
    pub conditions: Conditions,
    pub body: Vec<Element>,
}

/// Combined conditions on an `<if>` / `<else-if>`. CSL allows
/// multiple test attributes; the `match` attr controls how they
/// combine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conditions {
    /// `match="all" | "any" | "none"`.
    pub match_kind: MatchKind,
    pub variable: Vec<String>,
    pub item_type: Vec<String>,
    pub is_numeric: Vec<String>,
    pub is_uncertain_date: Vec<String>,
    pub locator: Vec<String>,
    pub position: Vec<String>,
    pub disambiguate: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatchKind {
    #[default]
    All,
    Any,
    None,
}

// ─────────────────────────────────────────────────────────────────
// Names + sub-elements
// ─────────────────────────────────────────────────────────────────

/// `<names>` — render one or more name variables with formatting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NamesElement {
    pub variables: Vec<String>,
    pub name: Option<NameElement>,
    pub et_al: Option<EtAlElement>,
    pub label: Option<LabelElement>,
    pub substitute: Vec<Element>,
    pub delimiter: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub formatting: Formatting,
    pub display: Option<Display>,
}

/// `<name>` — controls how each name is formatted. All optional
/// because `<names>` may inherit defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameElement {
    pub form: Option<NameForm>,
    pub and: Option<NameAnd>,
    pub delimiter: Option<String>,
    pub delimiter_precedes_et_al: Option<String>,
    pub delimiter_precedes_last: Option<String>,
    pub et_al_min: Option<u32>,
    pub et_al_use_first: Option<u32>,
    pub et_al_use_last: bool,
    pub initialize: bool,
    pub initialize_with: Option<String>,
    pub initialize_with_hyphen: bool,
    pub name_as_sort_order: Option<NameAsSortOrder>,
    pub sort_separator: Option<String>,
    pub formatting: Formatting,
    pub name_parts: Vec<NamePart>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameForm {
    Long,
    Short,
    Count,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameAnd {
    /// "Smith and Doe"
    Text,
    /// "Smith & Doe"
    Symbol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameAsSortOrder {
    First,
    All,
}

/// `<name-part name="family">` / `name="given"` — formatting per
/// name part (rare).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamePart {
    pub name: String,
    pub formatting: Formatting,
    pub text_case: Option<TextCase>,
}

/// `<et-al>` — controls the et-al term formatting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EtAlElement {
    pub term: Option<String>,
    pub formatting: Formatting,
}

// ─────────────────────────────────────────────────────────────────
// Date + DatePart
// ─────────────────────────────────────────────────────────────────

/// `<date>` — render a date variable. Either inline `date-part`
/// children OR `form="text"|"numeric"` referencing the locale's
/// date definition.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateElement {
    pub variable: String,
    pub form: Option<DateForm>,
    pub date_parts_filter: Option<DatePartsFilter>,
    pub date_parts: Vec<DatePart>,
    pub delimiter: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub formatting: Formatting,
    pub display: Option<Display>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateForm {
    Text,
    Numeric,
}

/// `<date date-parts="year-month-day">` filter — pin which parts to
/// emit when using `form="text"|"numeric"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePartsFilter {
    Year,
    YearMonth,
    YearMonthDay,
}

/// `<date-part>` — formatting for one part of a date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatePart {
    pub name: DatePartName,
    pub form: Option<DatePartForm>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub range_delimiter: Option<String>,
    pub formatting: Formatting,
    pub text_case: Option<TextCase>,
    pub strip_periods: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePartName {
    Year,
    Month,
    Day,
}

/// Forms a date-part can take depending on `name`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePartForm {
    Numeric,
    NumericLeadingZeros,
    Ordinal,
    /// `month` only: "January"
    Long,
    /// `month` only: "Jan."
    Short,
    /// `year` only: 2-digit
    Short2,
}

// ─────────────────────────────────────────────────────────────────
// Label element
// ─────────────────────────────────────────────────────────────────

/// `<label>` — render a label term (eg. "p." for page) for a
/// variable. Inside `<names>`, the label term comes from the names
/// variable name (eg. "editor"); outside, from the `variable` attr.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LabelElement {
    /// Required when not nested in `<names>`.
    pub variable: Option<String>,
    pub form: Option<String>,
    pub plural: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub strip_periods: bool,
    pub text_case: Option<TextCase>,
    pub formatting: Formatting,
}

// ─────────────────────────────────────────────────────────────────
// Locale + Term
// ─────────────────────────────────────────────────────────────────

/// A parsed CSL locale. Standalone files (`locales-en-US.xml`) and
/// in-style locale overrides share this shape.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Locale {
    /// `xml:lang` — eg. "en-US". `None` for in-style locales that
    /// apply to all languages.
    pub lang: Option<String>,
    pub style_options: StyleOptions,
    pub terms: Vec<Term>,
    /// `<date form="text">` and `<date form="numeric">` definitions.
    pub date_text: Option<DateElement>,
    pub date_numeric: Option<DateElement>,
}

/// `<style-options>` inside a locale.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleOptions {
    pub punctuation_in_quote: bool,
    pub limit_day_ordinals_to_day_1: bool,
}

/// `<term>` — locale-specific noun / verb / connective.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub name: String,
    pub form: TermForm,
    pub gender: Option<TermGender>,
    pub gender_form: Option<TermGender>,
    pub match_attr: Option<String>,
    pub body: TermBody,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TermForm {
    #[default]
    Long,
    Short,
    Verb,
    VerbShort,
    Symbol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermGender {
    Masculine,
    Feminine,
    Neuter,
}

/// Either flat content or `<single>` + `<multiple>` for plural
/// switching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermBody {
    Flat(String),
    Plural { single: String, multiple: String },
}
