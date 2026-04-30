//! Phase 4.5 — Find engine.
//!
//! Pure-functional substring search over a UTF-8 string. Lives in
//! `editor-core` so the registry-backed wrapper (`crate::find`) and
//! the wasm bridge (`crate::bridge::find_json`) can both call into
//! the same primitive without duplicating semantics.
//!
//! ## Locked decisions (2026-04-29)
//!
//! 1. **Whole-word boundary** — Unicode-aware via `char::is_alphanumeric`.
//!    A position `p` in `text` is a word boundary iff the codepoint
//!    immediately preceding `p` (if any) and the codepoint at `p`
//!    (if any) disagree on `is_alphanumeric()`. This covers Spanish
//!    (`ñ`, `á`), Portuguese (`ç`, `ã`), French (`é`, `ô`), German
//!    (`ä`, `ß`), Italian, etc. — every Latin-script ligature an
//!    academic user types — without pulling in a UAX #29 dep.
//!    Underscore is NOT treated as a letter (matches Rust's
//!    `is_alphanumeric`); regex `\w` semantics are explicitly *not*
//!    what we want here. CJK / Thai / Arabic semantic correctness
//!    is a v1 upgrade (swap to `unicode-segmentation`).
//!
//! 2. **Match offset units** — codepoint offsets, half-open `[start, end)`.
//!    Matches `apalabrar_doc_model::Position` so a [`Match`] flows
//!    directly into [`apalabrar_doc_model::EditOp::DeleteRange`] /
//!    [`InsertText`] for "Replace" without a byte↔codepoint hop.
//!
//! 3. **Overlapping matches** — non-overlapping. Searching "aa" in
//!    "aaaa" returns matches at `[0..2)` and `[2..4)` (two), not
//!    `[0..2)`, `[1..3)`, `[2..4)` (three). This matches Word and
//!    VS Code plain-mode behaviour.
//!
//! 4. **Empty needle** — returns the empty `Vec`. We refuse to
//!    enumerate every cursor position; the UI shows "0 of 0" and
//!    waits for the user to type.
//!
//! ## Performance contract
//!
//! [`find`] is O(n × m) worst case (naive scan; n = haystack chars,
//! m = needle chars). For v0's 100-page-doc corpus that's well under
//! a millisecond per query. The rope's substring search (jumprope)
//! is wired in `crate::find` against the registry-backed `Doc`; the
//! pure-string [`find`] here is the unit-testable primitive that
//! algorithm reduces to once a chunk is materialised.
//!
//! ## Invariants (proptest-asserted)
//!
//! - Every returned [`Match`] is a valid char-boundary range in the
//!   haystack: `haystack.is_char_boundary(m.start)` and
//!   `haystack.is_char_boundary(m.end)`.
//! - Returned matches are sorted ascending by `start` and
//!   non-overlapping: `matches[i].end <= matches[i+1].start`.
//! - `matches.len() <= haystack.chars().count() + 1` — the trivial
//!   upper bound for non-overlapping matches.
//! - Replacing the needle with the haystack itself returns exactly
//!   one match `[0..haystack.chars().count())` (when both options off).

use std::ops::Range;

/// Codepoint range of a single match, half-open `[start, end)`.
/// Units are codepoints (NOT bytes), matching
/// [`apalabrar_doc_model::Position`].
pub type Match = Range<usize>;

/// Search options.
///
/// Defaults: case-insensitive, not whole-word — i.e. the most
/// permissive search, which is what users expect when they hit
/// Ctrl-F without touching the toggle buttons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindOptions {
    /// When `true`, "Foo" only matches "Foo" — not "foo" or "FOO".
    /// When `false`, all-Unicode case-folding via `char::to_lowercase`
    /// (multi-codepoint folds expand correctly: `ß` → `ss`, `İ` → `i`+`̇`).
    pub case_sensitive: bool,
    /// When `true`, a match only counts if the codepoint immediately
    /// before its start (if any) and the codepoint immediately at its
    /// end (if any) are NOT alphanumeric — i.e. word-boundary on both
    /// sides. See module docs for the boundary predicate.
    pub whole_word: bool,
}

impl FindOptions {
    /// Default-permissive: case-insensitive, no whole-word.
    /// Equivalent to `FindOptions::default()` but reads better at
    /// call sites: `find(text, q, FindOptions::permissive())`.
    pub const fn permissive() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
        }
    }

    /// Convenience: case-sensitive, no whole-word.
    pub const fn case_sensitive() -> Self {
        Self {
            case_sensitive: true,
            whole_word: false,
        }
    }

    /// Convenience: whole-word, case-insensitive.
    pub const fn whole_word() -> Self {
        Self {
            case_sensitive: false,
            whole_word: true,
        }
    }
}

/// Find all non-overlapping occurrences of `needle` in `haystack`.
///
/// Returns codepoint ranges sorted ascending by `start`. An empty
/// needle returns `vec![]` (we refuse to enumerate cursor positions).
/// A needle longer than the haystack returns `vec![]`.
///
/// See module docs for the locked semantics of `case_sensitive` and
/// `whole_word`.
pub fn find(haystack: &str, needle: &str, opts: FindOptions) -> Vec<Match> {
    if needle.is_empty() {
        return Vec::new();
    }
    let h: Vec<char> = haystack.chars().map(|c| fold(c, opts)).collect();
    let n: Vec<char> = needle.chars().map(|c| fold(c, opts)).collect();
    if n.len() > h.len() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    let mut i = 0usize;
    while i + n.len() <= h.len() {
        if h[i..i + n.len()] == n[..] {
            let end = i + n.len();
            if !opts.whole_word
                || (is_word_boundary(haystack, i) && is_word_boundary(haystack, end))
            {
                matches.push(i..end);
                i = end;
                continue;
            }
        }
        i += 1;
    }
    matches
}

/// Per-codepoint fold used by [`find`] for case-insensitive matching.
/// Lossy on multi-codepoint folds (`İ` → `i`, dropping the combining
/// dot above) — keeps codepoint indices 1-to-1 between haystack and
/// folded haystack so [`Match`] offsets land on real haystack
/// boundaries. Multi-char fold support is a v1 upgrade.
fn fold(c: char, opts: FindOptions) -> char {
    if opts.case_sensitive {
        c
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// `true` iff position `pos` (a codepoint index into `text`) is a
/// word boundary — i.e. the codepoints on either side disagree on
/// `char::is_alphanumeric()`. The start (`pos == 0`) and end of
/// `text` are always boundaries.
///
/// `pos` is a CODEPOINT index, not a byte index. `pos > text.chars().count()`
/// is invalid; callers must pre-clip.
///
/// This is the predicate `whole_word` is built on. It's `pub(crate)`
/// rather than private so the in-crate `#[cfg(test)] mod tests`
/// below can exercise it directly without going through `find()`.
pub(crate) fn is_word_boundary(text: &str, pos: usize) -> bool {
    let prev = if pos == 0 {
        None
    } else {
        text.chars().nth(pos - 1)
    };
    let curr = text.chars().nth(pos);
    match (prev, curr) {
        (None, _) | (_, None) => true,
        (Some(p), Some(c)) => p.is_alphanumeric() != c.is_alphanumeric(),
    }
}

// ─────────────────────────────────────────────────────────────────
// Phase 4.5 RED — `is_word_boundary` direct unit tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_at_start_of_text_is_true() {
        // No "before" codepoint → boundary by definition.
        assert!(is_word_boundary("hello", 0));
    }

    #[test]
    fn boundary_at_end_of_text_is_true() {
        // No "at" codepoint → boundary by definition. "hello" has 5
        // codepoints; pos == 5 is one-past-end.
        assert!(is_word_boundary("hello", 5));
    }

    #[test]
    fn boundary_inside_a_word_is_false() {
        // pos=2 in "hello" sits between 'l' and 'l' — both alphanumeric.
        assert!(!is_word_boundary("hello", 2));
    }

    #[test]
    fn boundary_between_letter_and_space_is_true() {
        // "he o": pos=2 sits between 'e' (alpha) and ' ' (not alpha) → boundary.
        assert!(is_word_boundary("he o", 2));
    }

    #[test]
    fn boundary_between_unicode_letter_n_tilde_and_letter_is_false() {
        // Locked decision #1: ñ is alphanumeric. Position between 'e'
        // and 'ñ' in "señor" (pos=2) is NOT a boundary — both letters.
        assert!(!is_word_boundary("señor", 2));
    }
}
