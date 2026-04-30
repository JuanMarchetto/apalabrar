#![deny(unsafe_code)]
#![doc = "Citation engine — own-impl CSL 1.0.2 processor for academic citations and bibliographies."]

//! # Apalabrar citation engine (Phase 5.2)
//!
//! Pure-Rust implementation of CSL (Citation Style Language) 1.0.2.
//! Independent of `citeproc-rs` (MPL-2.0) — keeps the workspace
//! MIT-clean and avoids the upstream dormancy risk.
//!
//! ## Surface
//!
//! - [`render_bib`] — render a bibliography from `&[CslItem]`.
//! - [`render_inline`] — render a single in-text citation from `CslItem`.
//! - [`CslItem`] — CSL-JSON spec-compliant reference data; matches
//!   the shape Zotero / Mendeley export.
//! - [`OutputFormat`] — trait with [`Html`] + [`Plain`] impls; the
//!   short signatures default to HTML.
//! - [`Error`] — failure modes (unknown style/locale, malformed
//!   inputs).
//!
//! ## Bundled assets
//!
//! - 5 styles: APA 7, IEEE, MLA 9, AMA, Chicago notes-bibliography.
//! - 8 locales: en-US, en-GB, es-ES, pt-BR, de-DE, fr-FR, zh-CN, ru-RU.
//!
//! Style + locale XML files are CC-BY-SA 3.0; see `assets/NOTICE.md`.
//! The Rust code in this crate is MIT (workspace default).
//!
//! ## Deferred to Phase 5.2.x
//!
//! - Disambiguation (Smith 2020a vs Smith 2020b) — single-pass renderer.
//! - 5 additional styles (Harvard + AMA-equivs + Nature + Science +
//!   ACS) — Phase 5.2.1.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

pub mod assets;
pub mod ast;
pub mod ir;
pub mod parser;
pub mod renderer;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ─────────────────────────────────────────────────────────────────
// CSL-JSON reference (`CslItem`)
// ─────────────────────────────────────────────────────────────────

/// One bibliographic reference, shape-compatible with the CSL-JSON
/// schema (https://github.com/citation-style-language/schema). Most
/// fields are optional: a renderer that needs a missing field falls
/// back to the style's `substitute` chain or omits the variable.
///
/// This struct is passed by callers — typically deserialised from
/// JSON exported by Zotero / Mendeley / Better BibTeX without any
/// transformation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CslItem {
    /// Stable identifier (citekey). Used by `render_inline` to look
    /// up the rendered short-form citation.
    pub id: String,
    /// CSL item type ("article-journal", "book", "chapter", "thesis",
    /// "webpage", "report", etc.). See the CSL-JSON schema for the
    /// complete enum.
    #[serde(rename = "type")]
    pub item_type: String,
    /// Authors. Empty when the item has none (rare — institutional
    /// reports etc.); the renderer falls back to `substitute`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub author: Vec<NameVar>,
    /// Editors (book chapters, edited volumes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub editor: Vec<NameVar>,
    /// Translators.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub translator: Vec<NameVar>,
    /// Title of the work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Container title — journal, book series, anthology, conference.
    #[serde(
        rename = "container-title",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub container_title: Option<String>,
    /// Short / abbreviated container title (e.g. "J. Biol. Chem."
    /// for biomedical styles like AMA).
    #[serde(
        rename = "container-title-short",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub container_title_short: Option<String>,
    /// Publisher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// Publisher place ("New York", "London"). Common in book styles.
    #[serde(
        rename = "publisher-place",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub publisher_place: Option<String>,
    /// Date of issue / publication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issued: Option<DateVar>,
    /// Volume number ("12", "vol. 3").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range ("123-145" or "123").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// DOI.
    #[serde(rename = "DOI", default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// URL.
    #[serde(rename = "URL", default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// ISBN.
    #[serde(rename = "ISBN", default, skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    /// Edition ("2nd", "3").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Abstract (used by some styles in annotated bibliographies).
    #[serde(rename = "abstract", default, skip_serializing_if = "Option::is_none")]
    pub abstract_: Option<String>,
    /// Free-form catch-all for fields not yet modeled. CSL-JSON
    /// exports from Zotero often include extra metadata; we keep it
    /// to avoid losing data on round-trip.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// One personal or institutional name. Mirrors the CSL-JSON name
/// variable. Either `family` + (optionally) `given` are set for a
/// personal name, OR `literal` is set for an institutional name —
/// not both.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameVar {
    /// Family / surname.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Given name(s) ("John Q.").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    /// Particle that does NOT drop in alphabetisation ("van", "de la"
    /// for some traditions).
    #[serde(
        rename = "non-dropping-particle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub non_dropping_particle: Option<String>,
    /// Particle that DOES drop in alphabetisation.
    #[serde(
        rename = "dropping-particle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dropping_particle: Option<String>,
    /// Suffix ("Jr.", "III").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Institutional / corporate name. Mutually exclusive with
    /// `family`/`given`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub literal: Option<String>,
}

/// Date variable. CSL-JSON encodes dates as `{"date-parts": [[year,
/// month, day]]}` — we mirror that shape but expose helpers for the
/// renderer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateVar {
    /// Date parts: `[[year]]`, `[[year, month]]`, or `[[year, month,
    /// day]]`. A range uses `[[start...], [end...]]` (two entries).
    #[serde(rename = "date-parts", default, skip_serializing_if = "Vec::is_empty")]
    pub date_parts: Vec<Vec<i32>>,
    /// Free-form date literal ("Fall 2023") used when machine-readable
    /// parts aren't available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub literal: Option<String>,
    /// Indicates the date is approximate / circa.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub circa: bool,
}

// ─────────────────────────────────────────────────────────────────
// Output format trait
// ─────────────────────────────────────────────────────────────────

/// Pluggable serialiser for the renderer's intermediate
/// representation. Implementations decide how to wrap italic / bold /
/// link / superscript runs. The renderer never emits raw markup —
/// every formatting decision crosses this trait.
pub trait OutputFormat {
    /// Wrap `inner` as italic.
    fn italic(&self, inner: &str) -> String;
    /// Wrap `inner` as bold.
    fn bold(&self, inner: &str) -> String;
    /// Wrap `inner` as a hyperlink to `href`.
    fn link(&self, inner: &str, href: &str) -> String;
    /// Wrap `inner` as superscript (numeric biomedical citations).
    fn superscript(&self, inner: &str) -> String;
    /// Escape arbitrary text. For HTML, escapes `<>&"`; for plain,
    /// passes through unchanged.
    fn escape(&self, text: &str) -> String;
}

/// HTML output. Uses `<i>`, `<b>`, `<a href="…">`, `<sup>`. Escapes
/// `<>&"`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Html;

impl OutputFormat for Html {
    fn italic(&self, inner: &str) -> String {
        format!("<i>{inner}</i>")
    }
    fn bold(&self, inner: &str) -> String {
        format!("<b>{inner}</b>")
    }
    fn link(&self, inner: &str, href: &str) -> String {
        // href is also escaped to prevent attribute-injection.
        format!("<a href=\"{}\">{inner}</a>", self.escape(href))
    }
    fn superscript(&self, inner: &str) -> String {
        format!("<sup>{inner}</sup>")
    }
    fn escape(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            match c {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                '"' => out.push_str("&quot;"),
                _ => out.push(c),
            }
        }
        out
    }
}

/// Plain-text output — strips formatting. Useful for clipboard /
/// plain-text export.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plain;

impl OutputFormat for Plain {
    fn italic(&self, inner: &str) -> String {
        inner.to_string()
    }
    fn bold(&self, inner: &str) -> String {
        inner.to_string()
    }
    fn link(&self, inner: &str, _href: &str) -> String {
        // Plain text drops the URL; the visible label remains.
        inner.to_string()
    }
    fn superscript(&self, inner: &str) -> String {
        inner.to_string()
    }
    fn escape(&self, text: &str) -> String {
        text.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────
// Public render API
// ─────────────────────────────────────────────────────────────────

/// Render a bibliography from a slice of items. HTML output, en-US
/// locale. For other formats / locales use [`render_bib_with`].
///
/// `style` is one of the bundled style ids: `"apa"`, `"ieee"`,
/// `"mla"`, `"ama"`, `"chicago-notes-bibliography"`. Returns
/// [`Error::UnknownStyle`] otherwise.
pub fn render_bib(items: &[CslItem], style: &str) -> Result<String, Error> {
    render_bib_with(items, style, "en-US", &Html)
}

/// Render an in-text citation for a single item. HTML output, en-US.
pub fn render_inline(item: &CslItem, style: &str) -> Result<String, Error> {
    render_inline_with(item, style, "en-US", &Html)
}

/// Render a bibliography with explicit locale + output format.
pub fn render_bib_with<F: OutputFormat>(
    items: &[CslItem],
    style: &str,
    locale: &str,
    format: &F,
) -> Result<String, Error> {
    let parsed_style = compile_style(style)?;
    let parsed_locale = compile_locale(locale)?;
    if items.is_empty() {
        return Ok(String::new());
    }
    let layout = parsed_style
        .bibliography
        .as_ref()
        .map(|b| &b.layout)
        .unwrap_or(&parsed_style.citation.layout);
    let mut entries: Vec<String> = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        let mut ctx = renderer::RenderContext {
            style: &parsed_style,
            locale: &parsed_locale,
            item,
            position: idx + 1,
            macro_depth: 0,
            is_bibliography: true,
            group_scopes: Vec::new(),
        };
        let tokens = renderer::render_layout(&mut ctx, layout);
        entries.push(ir::serialize(&tokens, format));
    }
    Ok(entries.join("\n"))
}

/// Render an in-text citation with explicit locale + output format.
pub fn render_inline_with<F: OutputFormat>(
    item: &CslItem,
    style: &str,
    locale: &str,
    format: &F,
) -> Result<String, Error> {
    let parsed_style = compile_style(style)?;
    let parsed_locale = compile_locale(locale)?;
    let mut ctx = renderer::RenderContext {
        style: &parsed_style,
        locale: &parsed_locale,
        item,
        position: 1,
        macro_depth: 0,
        is_bibliography: false,
        group_scopes: Vec::new(),
    };
    let tokens = renderer::render_layout(&mut ctx, &parsed_style.citation.layout);
    Ok(ir::serialize(&tokens, format))
}

// ─────────────────────────────────────────────────────────────────
// Thread-local compile cache
// ─────────────────────────────────────────────────────────────────
//
// Phase 5.2-impl/locale: parsing a CSL XML is ~50ms cold (the APA
// bundle is 86KB); a long bibliography render with the same style
// would re-parse on every entry. Thread-local cache mirrors the
// `SHAPING_CACHE` pattern in `apalabrar-layout`.

thread_local! {
    static STYLE_CACHE: RefCell<HashMap<String, Rc<ast::Style>>> =
        RefCell::new(HashMap::new());
    static LOCALE_CACHE: RefCell<HashMap<String, Rc<ast::Locale>>> =
        RefCell::new(HashMap::new());
}

/// Compile a bundled style XML to AST. First call per id parses and
/// caches; subsequent calls return the cached `Rc` (zero-cost clone).
fn compile_style(id: &str) -> Result<Rc<ast::Style>, Error> {
    STYLE_CACHE.with(|cell| {
        if let Some(s) = cell.borrow().get(id).cloned() {
            return Ok(s);
        }
        let xml = assets::style_xml(id).ok_or_else(|| Error::UnknownStyle(id.into()))?;
        let parsed = parser::parse_style(xml, id).map_err(|e| Error::MalformedStyle {
            style: id.into(),
            reason: e.to_string(),
        })?;
        let rc = Rc::new(parsed);
        cell.borrow_mut().insert(id.to_string(), rc.clone());
        Ok(rc)
    })
}

/// Compile a bundled locale XML to AST. Same caching as
/// [`compile_style`].
fn compile_locale(id: &str) -> Result<Rc<ast::Locale>, Error> {
    LOCALE_CACHE.with(|cell| {
        if let Some(l) = cell.borrow().get(id).cloned() {
            return Ok(l);
        }
        let xml = assets::locale_xml(id).ok_or_else(|| Error::UnknownLocale(id.into()))?;
        let parsed = parser::parse_locale(xml).map_err(|e| Error::MalformedLocale {
            locale: id.into(),
            reason: e.to_string(),
        })?;
        let rc = Rc::new(parsed);
        cell.borrow_mut().insert(id.to_string(), rc.clone());
        Ok(rc)
    })
}

// ─────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────

/// Failure modes for the citation engine. Marked `#[non_exhaustive]`
/// so future phases can add variants (eg. `DisambiguationFailed`)
/// without a SemVer break.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// `style` is not one of the bundled ids.
    #[error("unknown CSL style: {0}")]
    UnknownStyle(String),
    /// `locale` is not one of the bundled ids.
    #[error("unknown CSL locale: {0}")]
    UnknownLocale(String),
    /// The bundled style XML failed to parse (compile-time bug —
    /// shouldn't happen for built-in assets, but surfaceable for
    /// future user-supplied styles).
    #[error("malformed CSL style ({style}): {reason}")]
    MalformedStyle { style: String, reason: String },
    /// The bundled locale XML failed to parse.
    #[error("malformed CSL locale ({locale}): {reason}")]
    MalformedLocale { locale: String, reason: String },
    /// A CSL feature the renderer doesn't yet support was used by
    /// the style (eg. `cs:bibliography subsequent-author-substitute`
    /// is deferred to Phase 5.2.x). Carries the feature name.
    #[error("unsupported CSL feature: {0}")]
    UnsupportedFeature(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
