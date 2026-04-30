//! Phase 5.2-impl/renderer-1 — CSL renderer (Style AST + CslItem → IR).
//!
//! Pipeline (per `<text>` element):
//!   1. resolve source (Variable / Term / Value / Macro)
//!   2. text-case
//!   3. strip-periods
//!   4. quotes
//!   5. formatting (italic / bold / etc., applied at serialize time)
//!   6. prefix / suffix (Group wrap)
//!   7. display
//!
//! Macro recursion is capped at [`MAX_MACRO_DEPTH`] to prevent
//! stack overflow on hostile / pathological style XML.

use crate::ast::*;
use crate::ir::{Token, transform_case};
use crate::{CslItem, DateVar, NameVar};

/// Hard cap on `<text macro="x"/>` expansion depth. Real-world CSL
/// styles nest at most 4-5 levels; 32 leaves ample headroom while
/// catching cycles.
pub const MAX_MACRO_DEPTH: usize = 32;

// ─────────────────────────────────────────────────────────────────
// Render context
// ─────────────────────────────────────────────────────────────────

/// Per-render state. Borrows the style + locale + item; carries
/// transient counters (current bibliography position, macro depth,
/// nested-group variable-call tracking).
pub struct RenderContext<'a> {
    pub style: &'a Style,
    pub locale: &'a Locale,
    pub item: &'a CslItem,
    /// 1-indexed position in the bibliography. `1` for inline render.
    pub position: usize,
    /// Current macro expansion depth.
    pub macro_depth: usize,
    /// Stack of `(variables_attempted, variables_rendered)` counts —
    /// one entry per open `<group>`. CSL group fail-if-empty is
    /// determined by VARIABLE call results: a group is suppressed
    /// when at least one variable was called and ALL variable calls
    /// returned empty. (Term / value calls do NOT count.)
    pub group_scopes: Vec<(usize, usize)>,
}

impl<'a> RenderContext<'a> {
    /// Record one variable resolution attempt + whether it produced
    /// content. Bumps the innermost group scope's counters.
    fn track_variable(&mut self, rendered: bool) {
        if let Some(scope) = self.group_scopes.last_mut() {
            scope.0 += 1;
            if rendered {
                scope.1 += 1;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Entry points
// ─────────────────────────────────────────────────────────────────

/// Render a `<layout>` body to IR. The layout's prefix/suffix/
/// delimiter are applied via the wrapping Group. Layout never
/// fail-if-empties — even an empty render emits the prefix/suffix.
pub fn render_layout(ctx: &mut RenderContext, layout: &Layout) -> Vec<Token> {
    let children = render_elements(ctx, &layout.body);
    if layout.prefix.is_none() && layout.suffix.is_none() && layout.delimiter.is_none() {
        return children;
    }
    vec![Token::Group {
        children,
        delimiter: layout.delimiter.clone(),
        prefix: layout.prefix.clone(),
        suffix: layout.suffix.clone(),
        formatting: layout.formatting.clone(),
        // Layout always renders, even when children are empty (CSL
        // distinguishes layout from group on this single point).
        fail_if_empty: false,
    }]
}

/// Render a list of `Element`s in order, flattening into a single
/// `Vec<Token>`. Used by layout / group / choose-branch / substitute.
pub fn render_elements(ctx: &mut RenderContext, body: &[Element]) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    for elem in body {
        out.extend(render_element(ctx, elem));
    }
    out
}

/// Dispatch to the right element renderer.
pub fn render_element(ctx: &mut RenderContext, elem: &Element) -> Vec<Token> {
    match elem {
        Element::Text(t) => render_text(ctx, t),
        Element::Number(n) => render_number(ctx, n),
        Element::Group(g) => render_group(ctx, g),
        Element::Choose(c) => render_choose(ctx, c),
        Element::Names(n) => render_names(ctx, n),
        Element::Date(d) => render_date(ctx, d),
        Element::Label(l) => render_label(ctx, l, None),
    }
}

// ─────────────────────────────────────────────────────────────────
// <text>
// ─────────────────────────────────────────────────────────────────

fn render_text(ctx: &mut RenderContext, text: &TextElement) -> Vec<Token> {
    let raw: Option<String> = match &text.source {
        TextSource::Variable(name) => {
            let v = resolve_variable(ctx, name);
            ctx.track_variable(v.is_some());
            v
        }
        TextSource::Macro(name) => return render_macro(ctx, name, text),
        TextSource::Term(name) => {
            let form = parse_term_form(text.form.as_deref());
            resolve_term(ctx, name, form, false)
        }
        TextSource::Value(s) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
    };
    let Some(mut s) = raw else {
        return Vec::new();
    };
    apply_inline_pipeline(&mut s, text.text_case, text.strip_periods, text.quotes);
    let inner = Token::Text {
        text: s,
        formatting: text.formatting.clone(),
    };
    wrap_with_affixes(inner, text.prefix.as_deref(), text.suffix.as_deref())
}

fn render_macro(ctx: &mut RenderContext, name: &str, text: &TextElement) -> Vec<Token> {
    if ctx.macro_depth >= MAX_MACRO_DEPTH {
        return Vec::new();
    }
    let Some(body) = ctx.style.macros.get(name).cloned() else {
        return Vec::new();
    };
    ctx.macro_depth += 1;
    let mut tokens = render_elements(ctx, &body);
    ctx.macro_depth -= 1;
    if tokens.iter().all(Token::is_empty) {
        return Vec::new();
    }
    // Apply text-case / strip-periods to the expanded macro output.
    if let Some(case) = text.text_case {
        for t in &mut tokens {
            t.apply_text_case(case);
        }
    }
    if text.strip_periods {
        for t in &mut tokens {
            t.strip_periods();
        }
    }
    // Apply formatting + affixes from the <text macro> wrapper.
    if !text.formatting.is_empty() || text.prefix.is_some() || text.suffix.is_some() {
        return vec![Token::Group {
            children: tokens,
            delimiter: None,
            prefix: text.prefix.clone(),
            suffix: text.suffix.clone(),
            formatting: text.formatting.clone(),
            fail_if_empty: true,
        }];
    }
    tokens
}

// ─────────────────────────────────────────────────────────────────
// <number>
// ─────────────────────────────────────────────────────────────────

fn render_number(ctx: &mut RenderContext, num: &NumberElement) -> Vec<Token> {
    let raw = resolve_variable(ctx, &num.variable);
    ctx.track_variable(raw.is_some());
    let Some(raw) = raw else {
        return Vec::new();
    };
    // Number form formatting (numeric / ordinal / long-ordinal /
    // roman). Phase 5.2-impl/renderer-1 ships pass-through (numeric);
    // ordinal / roman ship in renderer-2.
    let mut s = raw;
    if let Some(case) = num.text_case {
        s = transform_case(&s, case);
    }
    let inner = Token::Text {
        text: s,
        formatting: num.formatting.clone(),
    };
    wrap_with_affixes(inner, num.prefix.as_deref(), num.suffix.as_deref())
}

// ─────────────────────────────────────────────────────────────────
// <group>
// ─────────────────────────────────────────────────────────────────

fn render_group(ctx: &mut RenderContext, group: &GroupElement) -> Vec<Token> {
    ctx.group_scopes.push((0, 0));
    let children = render_elements(ctx, &group.body);
    let (attempted, rendered) = ctx.group_scopes.pop().unwrap_or((0, 0));
    // CSL group fail-if-empty: when at least one variable was called
    // and ALL variable calls returned empty, the entire group is
    // suppressed — including any term / value / delimiter / affix
    // children that did render. (CSL 1.0.2 §4.6).
    if attempted > 0 && rendered == 0 {
        // Bubble up: this nested group counts as one variable-attempt
        // failure for the parent group.
        if let Some(parent) = ctx.group_scopes.last_mut() {
            parent.0 += 1;
        }
        return Vec::new();
    }
    // Successful (or var-free) group bubbles up as a "rendered"
    // variable call so a parent group with this as its only variable-
    // bearing child is not erroneously suppressed.
    if attempted > 0
        && let Some(parent) = ctx.group_scopes.last_mut()
    {
        parent.0 += 1;
        parent.1 += 1;
    }
    if children.iter().all(Token::is_empty) {
        return Vec::new();
    }
    vec![Token::Group {
        children,
        delimiter: group.delimiter.clone(),
        prefix: group.prefix.clone(),
        suffix: group.suffix.clone(),
        formatting: group.formatting.clone(),
        fail_if_empty: true,
    }]
}

// ─────────────────────────────────────────────────────────────────
// <choose>
// ─────────────────────────────────────────────────────────────────

fn render_choose(ctx: &mut RenderContext, choose: &ChooseElement) -> Vec<Token> {
    for branch in &choose.branches {
        if evaluate_conditions(ctx, &branch.conditions) {
            return render_elements(ctx, &branch.body);
        }
    }
    render_elements(ctx, &choose.else_body)
}

fn evaluate_conditions(ctx: &RenderContext, c: &Conditions) -> bool {
    let checks: Vec<bool> = std::iter::empty()
        .chain(c.variable.iter().map(|v| variable_present(ctx, v)))
        .chain(c.item_type.iter().map(|t| ctx.item.item_type == *t))
        .chain(c.is_numeric.iter().map(|v| {
            resolve_variable(ctx, v)
                .map(|s| s.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        }))
        .chain(
            c.is_uncertain_date
                .iter()
                .map(|v| date_var(ctx.item, v).map(|d| d.circa).unwrap_or(false)),
        )
        // Position: Phase 5.2-impl/renderer-1 ships only "first" /
        // "subsequent" support (no last / ibid / near-note). For
        // single-item inline render: position = "first".
        .chain(c.position.iter().map(|p| match p.as_str() {
            "first" => ctx.position == 1,
            "subsequent" => ctx.position > 1,
            _ => false,
        }))
        // Locator support requires cite-cluster context; deferred.
        .chain(c.locator.iter().map(|_| false))
        // Disambiguate support deferred.
        .chain(std::iter::once(c.disambiguate).filter(|d| *d))
        .collect();
    if checks.is_empty() {
        return true; // No-condition branches always match.
    }
    match c.match_kind {
        MatchKind::All => checks.iter().all(|x| *x),
        MatchKind::Any => checks.iter().any(|x| *x),
        MatchKind::None => checks.iter().all(|x| !x),
    }
}

fn variable_present(ctx: &RenderContext, name: &str) -> bool {
    // For name vars, "present" means the vec has entries; for date
    // vars, the issued field is some; for plain string vars, the
    // resolver returns Some.
    match name {
        "author" => !ctx.item.author.is_empty(),
        "editor" => !ctx.item.editor.is_empty(),
        "translator" => !ctx.item.translator.is_empty(),
        "issued" => ctx.item.issued.is_some(),
        _ => resolve_variable(ctx, name).is_some(),
    }
}

fn date_var<'a>(item: &'a CslItem, name: &str) -> Option<&'a DateVar> {
    match name {
        "issued" => item.issued.as_ref(),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────
// <names>
// ─────────────────────────────────────────────────────────────────

fn render_names(ctx: &mut RenderContext, names: &NamesElement) -> Vec<Token> {
    // Pick the first non-empty variable. CSL says all listed vars
    // are joined with the names-delimiter, but in practice most
    // styles list only one variable per <names>; v0 picks the first.
    let mut rendered_var: Option<Vec<Token>> = None;
    let mut had_any_attempt = !names.variables.is_empty();
    for var in &names.variables {
        let people = name_var(ctx.item, var);
        if people.is_empty() {
            continue;
        }
        rendered_var = Some(render_one_names_var(ctx, names, var, people));
        break;
    }
    if let Some(t) = rendered_var {
        ctx.track_variable(true);
        return wrap_names_affixes(names, t);
    }
    if had_any_attempt {
        ctx.track_variable(false);
    }
    had_any_attempt = false; // suppress unused-warning shape
    let _ = had_any_attempt;
    // CSL `<substitute>`: try each child in order; first non-empty
    // wins, the rest are dropped. (CSL 1.0.2 §5.4.)
    for elem in &names.substitute {
        let tokens = render_element_with_inherited_name(ctx, elem, names.name.as_ref());
        if !tokens.iter().all(Token::is_empty) {
            return wrap_names_affixes(names, tokens);
        }
    }
    Vec::new()
}

/// Render a substitute body element. When the element is a nested
/// `<names>` with no `<name>` of its own, inherit the parent
/// `<name>` attrs — CSL's substitute inheritance rule (§5.4.1).
fn render_element_with_inherited_name(
    ctx: &mut RenderContext,
    elem: &Element,
    parent_name: Option<&NameElement>,
) -> Vec<Token> {
    if let Element::Names(child) = elem
        && child.name.is_none()
        && let Some(p) = parent_name
    {
        let mut inherited = (**child).clone();
        inherited.name = Some(p.clone());
        return render_names(ctx, &inherited);
    }
    render_element(ctx, elem)
}

fn render_one_names_var(
    ctx: &mut RenderContext,
    names: &NamesElement,
    var: &str,
    people: Vec<NameVar>,
) -> Vec<Token> {
    let name = names.name.as_ref();
    let total = people.len();
    let (et_al_min, et_al_use_first) =
        et_al_thresholds(name, names.formatting.clone(), &ctx.style.citation);
    let (visible_count, use_et_al) = match (et_al_min, et_al_use_first) {
        (Some(min), Some(first)) if total as u32 >= min => (first as usize, true),
        _ => (total, false),
    };
    let formatted_names: Vec<String> = people
        .iter()
        .take(visible_count)
        .map(|n| format_one_name(name, n))
        .collect();
    let connector_and = name.and_then(|n| n.and).unwrap_or(NameAnd::Text);
    let inter_delim = name
        .and_then(|n| n.delimiter.clone())
        .unwrap_or_else(|| ", ".into());
    let mut joined = String::new();
    for (i, fn_str) in formatted_names.iter().enumerate() {
        if i > 0 {
            if i == formatted_names.len() - 1 && !use_et_al {
                // Last name: use "and" separator.
                let connector = match connector_and {
                    NameAnd::Text => resolve_term(ctx, "and", TermForm::Long, false)
                        .unwrap_or_else(|| "and".into()),
                    NameAnd::Symbol => "&".into(),
                };
                joined.push_str(&inter_delim);
                joined.push_str(&connector);
                joined.push(' ');
            } else {
                joined.push_str(&inter_delim);
            }
        }
        joined.push_str(fn_str);
    }
    if use_et_al {
        let et_al_term = names
            .et_al
            .as_ref()
            .and_then(|e| e.term.clone())
            .unwrap_or_else(|| "et-al".into());
        let et_al_text = resolve_term(ctx, &et_al_term, TermForm::Long, false)
            .unwrap_or_else(|| "et al.".into());
        joined.push_str(&inter_delim);
        joined.push_str(&et_al_text);
    }
    let _ = var; // silence unused — future-use for var-specific formatting
    vec![Token::Text {
        text: joined,
        formatting: names.formatting.clone(),
    }]
}

fn et_al_thresholds(
    name: Option<&NameElement>,
    _names_fmt: Formatting,
    citation: &Citation,
) -> (Option<u32>, Option<u32>) {
    let from_name = name.and_then(|n| n.et_al_min).or(citation.et_al_min);
    let from_use_first = name
        .and_then(|n| n.et_al_use_first)
        .or(citation.et_al_use_first);
    (from_name, from_use_first)
}

fn format_one_name(name: Option<&NameElement>, n: &NameVar) -> String {
    if let Some(literal) = &n.literal {
        return literal.clone();
    }
    let family = n.family.as_deref().unwrap_or("");
    // form="short" emits only the family name (CSL 1.0.2 §5.4).
    if name.and_then(|nm| nm.form) == Some(NameForm::Short) {
        let particle = n.non_dropping_particle.as_deref().unwrap_or("").trim();
        if particle.is_empty() {
            return family.to_string();
        }
        return format!("{particle} {family}");
    }
    let given_full = n.given.as_deref().unwrap_or("");
    let initialize = name
        .and_then(|nm| nm.initialize_with.clone())
        .or_else(|| name.map(|_| "".to_string()));
    let given_out = if let Some(initialize_with) = &initialize {
        if !initialize_with.is_empty() {
            initialize_given_name(given_full, initialize_with)
        } else {
            given_full.to_string()
        }
    } else {
        given_full.to_string()
    };
    let sort_separator = name
        .and_then(|nm| nm.sort_separator.clone())
        .unwrap_or_else(|| ", ".into());
    let sort_order = name.and_then(|nm| nm.name_as_sort_order);
    let mut out = String::new();
    let particle_non_drop = n.non_dropping_particle.as_deref().unwrap_or("").trim();
    match sort_order {
        Some(NameAsSortOrder::First) | Some(NameAsSortOrder::All) => {
            // Family, Given (sort order)
            if !particle_non_drop.is_empty() {
                out.push_str(particle_non_drop);
                out.push(' ');
            }
            out.push_str(family);
            if !given_out.is_empty() {
                out.push_str(&sort_separator);
                out.push_str(&given_out);
            }
        }
        None => {
            // Given Family (display order)
            if !given_out.is_empty() {
                out.push_str(&given_out);
                out.push(' ');
            }
            if !particle_non_drop.is_empty() {
                out.push_str(particle_non_drop);
                out.push(' ');
            }
            out.push_str(family);
        }
    }
    if let Some(suffix) = &n.suffix {
        out.push_str(", ");
        out.push_str(suffix);
    }
    out
}

/// Convert a given name into initials, separated by `with` between
/// each initial. "John Quincy" + ". " → "J. Q.".
fn initialize_given_name(given: &str, with: &str) -> String {
    let mut out = String::new();
    for word in given.split_whitespace() {
        if let Some(c) = word.chars().next() {
            for cc in c.to_uppercase() {
                out.push(cc);
            }
            out.push_str(with);
        }
    }
    // Trim trailing whitespace from `with` if it ended with a space.
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

fn name_var(item: &CslItem, name: &str) -> Vec<NameVar> {
    match name {
        "author" => item.author.clone(),
        "editor" => item.editor.clone(),
        "translator" => item.translator.clone(),
        _ => Vec::new(),
    }
}

fn wrap_names_affixes(names: &NamesElement, inner: Vec<Token>) -> Vec<Token> {
    if names.prefix.is_none() && names.suffix.is_none() && names.delimiter.is_none() {
        return inner;
    }
    vec![Token::Group {
        children: inner,
        delimiter: names.delimiter.clone(),
        prefix: names.prefix.clone(),
        suffix: names.suffix.clone(),
        formatting: Formatting::default(),
        fail_if_empty: true,
    }]
}

// ─────────────────────────────────────────────────────────────────
// <date>
// ─────────────────────────────────────────────────────────────────

fn render_date(ctx: &mut RenderContext, date: &DateElement) -> Vec<Token> {
    let Some(d) = date_var(ctx.item, &date.variable) else {
        ctx.track_variable(false);
        return Vec::new();
    };
    ctx.track_variable(true);
    let parts = d.date_parts.first();
    let Some(parts) = parts else {
        // Literal fallback.
        if let Some(lit) = &d.literal {
            return wrap_with_affixes(
                Token::Text {
                    text: lit.clone(),
                    formatting: date.formatting.clone(),
                },
                date.prefix.as_deref(),
                date.suffix.as_deref(),
            );
        }
        return Vec::new();
    };
    // Build per-part rendering using either the inline date-parts OR
    // the locale's date definition (when form="text" / "numeric").
    let inline = !date.date_parts.is_empty();
    let mut chunks: Vec<String> = Vec::new();
    if inline {
        for part in &date.date_parts {
            if let Some(s) = render_date_part(part, parts, &date.date_parts_filter) {
                chunks.push(s);
            }
        }
    } else if let Some(_form) = date.form {
        // Locale-driven: look up the locale's <date form="text|numeric">
        // and render its date-parts. Phase 5.2-impl/renderer-1 ships
        // a minimal default if the locale doesn't define one.
        let locale_date = match date.form {
            Some(DateForm::Text) => ctx.locale.date_text.as_ref(),
            Some(DateForm::Numeric) => ctx.locale.date_numeric.as_ref(),
            None => None,
        };
        if let Some(loc_date) = locale_date {
            for part in &loc_date.date_parts {
                if let Some(s) = render_date_part(part, parts, &date.date_parts_filter) {
                    chunks.push(s);
                }
            }
        } else {
            // Default: year only.
            chunks.push(parts.first().copied().unwrap_or(0).to_string());
        }
    } else {
        // No form, no inline parts — emit year only as last-resort.
        if let Some(year) = parts.first() {
            chunks.push(year.to_string());
        }
    }
    let s = chunks.concat();
    if s.is_empty() {
        return Vec::new();
    }
    let inner = Token::Text {
        text: s,
        formatting: date.formatting.clone(),
    };
    wrap_with_affixes(inner, date.prefix.as_deref(), date.suffix.as_deref())
}

fn render_date_part(
    part: &DatePart,
    parts: &[i32],
    filter: &Option<DatePartsFilter>,
) -> Option<String> {
    let allowed = match (part.name, filter.unwrap_or(DatePartsFilter::YearMonthDay)) {
        (DatePartName::Year, _) => true,
        (DatePartName::Month, DatePartsFilter::Year) => false,
        (DatePartName::Month, _) => parts.len() >= 2,
        (DatePartName::Day, DatePartsFilter::Year)
        | (DatePartName::Day, DatePartsFilter::YearMonth) => false,
        (DatePartName::Day, _) => parts.len() >= 3,
    };
    if !allowed {
        return None;
    }
    let value = match part.name {
        DatePartName::Year => parts.first().copied().unwrap_or(0),
        DatePartName::Month => parts.get(1).copied().unwrap_or(0),
        DatePartName::Day => parts.get(2).copied().unwrap_or(0),
    };
    if value == 0 {
        return None;
    }
    let body = match (part.name, part.form.unwrap_or(DatePartForm::Numeric)) {
        (DatePartName::Year, _) => value.to_string(),
        (DatePartName::Month, DatePartForm::Long) => english_month_long(value),
        (DatePartName::Month, DatePartForm::Short) => english_month_short(value),
        (DatePartName::Month, _) => value.to_string(),
        (DatePartName::Day, DatePartForm::Ordinal) => {
            format!("{value}{}", english_ordinal_suffix(value))
        }
        (DatePartName::Day, _) => value.to_string(),
    };
    let mut s = String::new();
    if let Some(p) = &part.prefix {
        s.push_str(p);
    }
    s.push_str(&body);
    if let Some(suf) = &part.suffix {
        s.push_str(suf);
    }
    Some(s)
}

fn english_month_long(m: i32) -> String {
    let names = [
        "",
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    names.get(m as usize).copied().unwrap_or("").to_string()
}

fn english_month_short(m: i32) -> String {
    let names = [
        "", "Jan.", "Feb.", "Mar.", "Apr.", "May", "Jun.", "Jul.", "Aug.", "Sep.", "Oct.", "Nov.",
        "Dec.",
    ];
    names.get(m as usize).copied().unwrap_or("").to_string()
}

fn english_ordinal_suffix(n: i32) -> &'static str {
    let m = n.unsigned_abs() % 100;
    if (11..=13).contains(&m) {
        return "th";
    }
    match m % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

// ─────────────────────────────────────────────────────────────────
// <label>
// ─────────────────────────────────────────────────────────────────

fn render_label(
    ctx: &mut RenderContext,
    label: &LabelElement,
    inherited_var: Option<&str>,
) -> Vec<Token> {
    let var = label.variable.as_deref().or(inherited_var).unwrap_or("");
    if var.is_empty() {
        return Vec::new();
    }
    let plural = label_is_plural(ctx.item, var, label.plural.as_deref());
    let form = parse_term_form(label.form.as_deref());
    let Some(text) = resolve_term(ctx, var, form, plural) else {
        return Vec::new();
    };
    let mut s = text;
    if label.strip_periods {
        s = s.replace('.', "");
    }
    if let Some(case) = label.text_case {
        s = transform_case(&s, case);
    }
    let inner = Token::Text {
        text: s,
        formatting: label.formatting.clone(),
    };
    wrap_with_affixes(inner, label.prefix.as_deref(), label.suffix.as_deref())
}

fn label_is_plural(item: &CslItem, var: &str, plural_attr: Option<&str>) -> bool {
    match plural_attr {
        Some("always") => true,
        Some("never") => false,
        _ => match var {
            "author" => item.author.len() > 1,
            "editor" => item.editor.len() > 1,
            "translator" => item.translator.len() > 1,
            "page" => item
                .page
                .as_deref()
                .map(|p| p.contains('-') || p.contains(','))
                .unwrap_or(false),
            _ => false,
        },
    }
}

// ─────────────────────────────────────────────────────────────────
// Resolution helpers
// ─────────────────────────────────────────────────────────────────

/// Resolve a CSL string variable name against the context's item.
/// Returns `None` for missing or empty variables (CSL "available"
/// semantics).
pub fn resolve_variable(ctx: &RenderContext, name: &str) -> Option<String> {
    let raw: Option<String> = match name {
        "title" => ctx.item.title.clone(),
        "title-short" => ctx.item.title.clone(), // Most styles fall back to title.
        "container-title" => ctx.item.container_title.clone(),
        "container-title-short" => ctx.item.container_title_short.clone(),
        "publisher" => ctx.item.publisher.clone(),
        "publisher-place" => ctx.item.publisher_place.clone(),
        "volume" => ctx.item.volume.clone(),
        "issue" => ctx.item.issue.clone(),
        "page" => ctx.item.page.clone(),
        "page-first" => ctx
            .item
            .page
            .as_deref()
            .map(|p| p.split(['-', ',', ' ']).next().unwrap_or(p).to_string()),
        "DOI" => ctx.item.doi.clone(),
        "URL" => ctx.item.url.clone(),
        "ISBN" => ctx.item.isbn.clone(),
        "edition" => ctx.item.edition.clone(),
        "abstract" => ctx.item.abstract_.clone(),
        "id" | "citation-key" => Some(ctx.item.id.clone()),
        "type" => Some(ctx.item.item_type.clone()),
        "citation-number" => Some(ctx.position.to_string()),
        _ => ctx
            .item
            .extra
            .get(name)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
    };
    raw.filter(|s| !s.is_empty())
}

/// Resolve a CSL term to its localised text.
pub fn resolve_term(
    ctx: &RenderContext,
    name: &str,
    form: TermForm,
    plural: bool,
) -> Option<String> {
    // Try the requested form, then fall back to Long.
    let forms = if form == TermForm::Long {
        vec![TermForm::Long]
    } else {
        vec![form, TermForm::Long]
    };
    for f in forms {
        // In-style locale overrides take precedence — they apply to
        // ANY language (lang=None) or the matching language.
        let style_lang = ctx.locale.lang.as_deref();
        for ovr in &ctx.style.locale_overrides {
            if ovr.lang.is_none() || ovr.lang.as_deref() == style_lang {
                if let Some(t) = ovr.terms.iter().find(|t| t.name == name && t.form == f) {
                    return Some(term_body_to_string(&t.body, plural));
                }
            }
        }
        if let Some(t) = ctx
            .locale
            .terms
            .iter()
            .find(|t| t.name == name && t.form == f)
        {
            return Some(term_body_to_string(&t.body, plural));
        }
    }
    None
}

fn term_body_to_string(body: &TermBody, plural: bool) -> String {
    match body {
        TermBody::Flat(s) => s.clone(),
        TermBody::Plural { single, multiple } => {
            if plural {
                multiple.clone()
            } else {
                single.clone()
            }
        }
    }
}

fn parse_term_form(form: Option<&str>) -> TermForm {
    match form {
        Some("short") => TermForm::Short,
        Some("verb") => TermForm::Verb,
        Some("verb-short") => TermForm::VerbShort,
        Some("symbol") => TermForm::Symbol,
        _ => TermForm::Long,
    }
}

// ─────────────────────────────────────────────────────────────────
// Inline pipeline + affix wrap
// ─────────────────────────────────────────────────────────────────

fn apply_inline_pipeline(
    s: &mut String,
    text_case: Option<TextCase>,
    strip_periods: bool,
    quotes: bool,
) {
    if let Some(case) = text_case {
        *s = transform_case(s, case);
    }
    if strip_periods {
        *s = s.replace('.', "");
    }
    if quotes {
        *s = format!("\u{201C}{}\u{201D}", s); // curly double quotes
    }
}

fn wrap_with_affixes(inner: Token, prefix: Option<&str>, suffix: Option<&str>) -> Vec<Token> {
    if prefix.is_none() && suffix.is_none() {
        return vec![inner];
    }
    vec![Token::Group {
        children: vec![inner],
        delimiter: None,
        prefix: prefix.map(String::from),
        suffix: suffix.map(String::from),
        formatting: Formatting::default(),
        fail_if_empty: true,
    }]
}

// ─────────────────────────────────────────────────────────────────
// Helpers for ast::Formatting
// ─────────────────────────────────────────────────────────────────

trait FormattingExt {
    fn is_empty(&self) -> bool;
}

impl FormattingExt for Formatting {
    fn is_empty(&self) -> bool {
        self.font_style.is_none()
            && self.font_weight.is_none()
            && self.font_variant.is_none()
            && self.text_decoration.is_none()
            && self.vertical_align.is_none()
    }
}
