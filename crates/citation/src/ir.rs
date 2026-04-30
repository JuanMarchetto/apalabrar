//! Phase 5.2-impl/renderer — intermediate representation.
//!
//! The renderer produces a token tree (not a flat string) so that
//! `text-case` and `strip-periods` can transform leaf text without
//! touching markup, and `<group>` fail-if-empty semantics can be
//! computed by checking child text emptiness directly.
//!
//! See module-level doc on the renderer for pipeline order:
//! resolve → text-case → strip-periods → quotes → formatting →
//! prefix/suffix → display.

use crate::OutputFormat;
use crate::ast::*;

/// One node of the renderer IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// Atomic text span carrying its own formatting. The serialize
    /// step calls `OutputFormat::escape` + the appropriate wrappers
    /// (italic/bold/superscript) when emitting this node.
    Text {
        text: String,
        formatting: Formatting,
    },
    /// Nested wrapper. Children are joined by `delimiter` (if any),
    /// then wrapped in `prefix` + `suffix`. The whole group is
    /// formatted as `formatting` after concatenation.
    ///
    /// If `fail_if_empty` and every child is text-empty, the group
    /// (along with its delimiter / affixes) is suppressed entirely
    /// — CSL `<group>` semantics.
    Group {
        children: Vec<Token>,
        delimiter: Option<String>,
        prefix: Option<String>,
        suffix: Option<String>,
        formatting: Formatting,
        fail_if_empty: bool,
    },
    /// A hyperlink span. Paint the visible label using `formatting`
    /// inside an `<a href>`. Plain output drops the URL.
    Link {
        label: String,
        href: String,
        formatting: Formatting,
    },
}

impl Token {
    /// `true` when this token contributes no visible text. Used by
    /// fail-if-empty groups and by serialize to skip empty children
    /// before applying delimiters.
    ///
    /// A text token is empty when its text is empty. A group is
    /// empty when every child is empty. A link is empty when its
    /// label is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Token::Text { text, .. } => text.is_empty(),
            Token::Group { children, .. } => children.iter().all(Token::is_empty),
            Token::Link { label, .. } => label.is_empty(),
        }
    }

    /// Apply `case` to every leaf text in this token tree. Idempotent
    /// for already-cased text; preserves leaf formatting metadata
    /// (italics on a journal title stay italic when title-cased).
    pub fn apply_text_case(&mut self, case: TextCase) {
        match self {
            Token::Text { text, .. } => *text = transform_case(text, case),
            Token::Group { children, .. } => {
                for c in children {
                    c.apply_text_case(case);
                }
            }
            Token::Link { label, .. } => *label = transform_case(label, case),
        }
    }

    /// Remove all `.` characters from every leaf text. Applied AFTER
    /// `text-case` per pipeline order — stripping first would erase
    /// word boundaries that title-case relies on.
    pub fn strip_periods(&mut self) {
        match self {
            Token::Text { text, .. } => *text = text.replace('.', ""),
            Token::Group { children, .. } => {
                for c in children {
                    c.strip_periods();
                }
            }
            Token::Link { label, .. } => *label = label.replace('.', ""),
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Serialize: IR → String via OutputFormat
// ─────────────────────────────────────────────────────────────────

/// Walk the IR and emit a single string via the supplied output
/// format. Empty groups (and fail-if-empty children) are skipped;
/// delimiters appear only between non-empty children.
///
/// Applies CSL punctuation-collapse post-processing: adjacent
/// terminal punctuation (`. .`, `,.`, `?.`) collapses to a single
/// terminator. citeproc-js + citeproc-rs both do this; without it,
/// `initialize-with=". "` colliding with a `delimiter=". "` produces
/// double-period output.
pub fn serialize<F: OutputFormat>(tokens: &[Token], format: &F) -> String {
    let mut out = String::new();
    serialize_into(tokens, format, &mut out);
    collapse_punctuation(&out)
}

fn collapse_punctuation(s: &str) -> String {
    // Order matters: collapse the longer patterns first so a triple-
    // period collapses cleanly.
    s.replace(".. ", ". ")
        .replace("..\n", ".\n")
        .replace(",.", ".")
        .replace(",,", ",")
        .replace("?.", "?")
        .replace("!.", "!")
        // Trim a trailing "..".
        .trim_end_matches('.')
        .to_string()
        + if s.ends_with('.') { "." } else { "" }
}

fn serialize_into<F: OutputFormat>(tokens: &[Token], format: &F, out: &mut String) {
    for t in tokens {
        match t {
            Token::Text { text, formatting } => {
                if text.is_empty() {
                    continue;
                }
                let escaped = format.escape(text);
                out.push_str(&apply_formatting(&escaped, formatting, format));
            }
            Token::Group {
                children,
                delimiter,
                prefix,
                suffix,
                formatting,
                fail_if_empty,
            } => {
                if *fail_if_empty && t.is_empty() {
                    continue;
                }
                let parts: Vec<String> = children
                    .iter()
                    .filter(|c| !c.is_empty())
                    .map(|c| serialize(std::slice::from_ref(c), format))
                    .collect();
                let body = match delimiter {
                    Some(d) => parts.join(d),
                    None => parts.concat(),
                };
                let formatted = apply_formatting(&body, formatting, format);
                if let Some(p) = prefix {
                    out.push_str(p);
                }
                out.push_str(&formatted);
                if let Some(s) = suffix {
                    out.push_str(s);
                }
            }
            Token::Link {
                label,
                href,
                formatting,
            } => {
                if label.is_empty() {
                    continue;
                }
                let escaped = format.escape(label);
                let formatted = apply_formatting(&escaped, formatting, format);
                out.push_str(&format.link(&formatted, href));
            }
        }
    }
}

fn apply_formatting<F: OutputFormat>(s: &str, f: &Formatting, format: &F) -> String {
    let mut out = s.to_string();
    if matches!(
        f.font_style,
        Some(FontStyle::Italic) | Some(FontStyle::Oblique)
    ) {
        out = format.italic(&out);
    }
    if f.font_weight == Some(FontWeight::Bold) {
        out = format.bold(&out);
    }
    if f.vertical_align == Some(VerticalAlign::Sup) {
        out = format.superscript(&out);
    }
    out
}

/// Apply a `TextCase` transform to a string. Pure function; no
/// locale awareness (CSL's text-case isn't locale-aware in v1.0.2).
pub fn transform_case(s: &str, case: TextCase) -> String {
    match case {
        TextCase::Lowercase => s.to_lowercase(),
        TextCase::Uppercase => s.to_uppercase(),
        TextCase::CapitalizeFirst => capitalize_first(s),
        TextCase::Capitalize => capitalize_each_word(s),
        TextCase::Sentence => sentence_case(s),
        TextCase::Title => title_case(s),
    }
}

fn capitalize_first(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    if let Some(c) = chars.next() {
        out.extend(c.to_uppercase());
    }
    out.push_str(chars.as_str());
    out
}

fn capitalize_each_word(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for c in s.chars() {
        if c.is_whitespace() {
            at_word_start = true;
            out.push(c);
        } else if at_word_start {
            out.extend(c.to_uppercase());
            at_word_start = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn sentence_case(s: &str) -> String {
    // First letter uppercase, rest lowercase.
    let mut chars = s.chars();
    let mut out = String::with_capacity(s.len());
    if let Some(c) = chars.next() {
        out.extend(c.to_uppercase());
    }
    for c in chars {
        out.extend(c.to_lowercase());
    }
    out
}

/// English stop words kept lowercase by `title-case` (mid-string).
/// First word + word-after-colon are always capitalised regardless.
/// Citeproc-js uses this same list; we mirror it for cross-engine
/// compatibility.
const TITLE_CASE_STOP_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "down", "for", "from", "if", "in", "into", "nor",
    "of", "on", "onto", "or", "out", "over", "so", "the", "till", "to", "up", "via", "with", "yet",
];

fn title_case(s: &str) -> String {
    // Split into whitespace-delimited words; capitalise first +
    // last + non-stop-word; preserve original case of stop words
    // mid-string. Hyphens treated as word boundaries (English
    // typographic convention).
    let words: Vec<&str> = s.split_whitespace().collect();
    let last = words.len().saturating_sub(1);
    let mut out = String::with_capacity(s.len());
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let lower = word.to_lowercase();
        let is_stop = TITLE_CASE_STOP_WORDS.contains(&lower.as_str());
        if i == 0 || i == last || !is_stop {
            out.push_str(&capitalize_first(word));
        } else {
            out.push_str(&lower);
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────
// Tests for IR helpers
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Token {
        Token::Text {
            text: s.into(),
            formatting: Formatting::default(),
        }
    }

    #[test]
    fn text_token_is_empty_when_text_is_empty() {
        assert!(t("").is_empty());
        assert!(!t("x").is_empty());
    }

    #[test]
    fn group_is_empty_when_all_children_empty() {
        let g = Token::Group {
            children: vec![t(""), t("")],
            delimiter: Some(", ".into()),
            prefix: None,
            suffix: None,
            formatting: Formatting::default(),
            fail_if_empty: true,
        };
        assert!(g.is_empty());
    }

    #[test]
    fn group_is_not_empty_when_one_child_has_text() {
        let g = Token::Group {
            children: vec![t(""), t("x")],
            delimiter: None,
            prefix: None,
            suffix: None,
            formatting: Formatting::default(),
            fail_if_empty: true,
        };
        assert!(!g.is_empty());
    }

    #[test]
    fn link_is_empty_when_label_is_empty() {
        let l = Token::Link {
            label: String::new(),
            href: "http://x".into(),
            formatting: Formatting::default(),
        };
        assert!(l.is_empty());
    }

    #[test]
    fn apply_text_case_uppercase_transforms_leaves() {
        let mut tok = t("hello world");
        tok.apply_text_case(TextCase::Uppercase);
        let Token::Text { text, .. } = &tok else {
            panic!()
        };
        assert_eq!(text, "HELLO WORLD");
    }

    #[test]
    fn apply_text_case_capitalize_each_word() {
        let mut tok = t("hello world foo");
        tok.apply_text_case(TextCase::Capitalize);
        let Token::Text { text, .. } = &tok else {
            panic!()
        };
        assert_eq!(text, "Hello World Foo");
    }

    #[test]
    fn apply_text_case_capitalize_first_only() {
        let mut tok = t("hello world");
        tok.apply_text_case(TextCase::CapitalizeFirst);
        let Token::Text { text, .. } = &tok else {
            panic!()
        };
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn apply_text_case_sentence() {
        let mut tok = t("HELLO WORLD");
        tok.apply_text_case(TextCase::Sentence);
        let Token::Text { text, .. } = &tok else {
            panic!()
        };
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn strip_periods_removes_all_dots_in_leaves() {
        let mut tok = t("U.K. of G.B.");
        tok.strip_periods();
        let Token::Text { text, .. } = &tok else {
            panic!()
        };
        assert_eq!(text, "UK of GB");
    }

    #[test]
    fn pipeline_order_text_case_then_strip_periods_preserves_acronyms() {
        // Pipeline order: text-case BEFORE strip-periods.
        // "U.K." with title-case → "U.K." (no change, all caps already
        // capitalized) → strip → "UK". Doing it the other way ("UK"
        // → titlecase → "Uk") would lose the acronym.
        let mut tok = t("U.K.");
        tok.apply_text_case(TextCase::Capitalize);
        tok.strip_periods();
        let Token::Text { text, .. } = &tok else {
            panic!()
        };
        assert_eq!(text, "UK");
    }

    #[test]
    fn apply_text_case_recurses_into_groups_preserving_formatting() {
        let italic = Formatting {
            font_style: Some(FontStyle::Italic),
            ..Formatting::default()
        };
        let mut g = Token::Group {
            children: vec![Token::Text {
                text: "hello".into(),
                formatting: italic,
            }],
            delimiter: None,
            prefix: None,
            suffix: None,
            formatting: Formatting::default(),
            fail_if_empty: false,
        };
        g.apply_text_case(TextCase::Uppercase);
        let Token::Group { children, .. } = &g else {
            panic!()
        };
        let Token::Text { text, formatting } = &children[0] else {
            panic!()
        };
        assert_eq!(text, "HELLO");
        // Italic preserved through case transform.
        assert_eq!(formatting.font_style, Some(FontStyle::Italic));
    }
}
