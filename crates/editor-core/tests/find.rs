//! RED-phase tests for the Phase 4.5 find engine.
//!
//! All tests target `apalabrar_editor_core::find::find` directly — the
//! pure-functional substring-search primitive that the registry-backed
//! `crate::find` and the wasm `findInDoc` both reduce to.
//!
//! Locked semantics (see `crates/editor-core/src/find.rs` module docs):
//! - codepoint (NOT byte) offsets in `Match`
//! - `whole_word` boundary = Unicode `is_alphanumeric()` disagreement
//! - non-overlapping matches
//! - empty needle → empty Vec

use apalabrar_editor_core::find::{FindOptions, Match, find};
use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────
// Trivial / empty inputs
// ─────────────────────────────────────────────────────────────────

#[test]
fn returns_empty_vec_when_haystack_is_empty() {
    assert_eq!(
        find("", "needle", FindOptions::permissive()),
        Vec::<Match>::new()
    );
}

#[test]
fn returns_empty_vec_when_needle_is_empty() {
    // Locked decision #4: we refuse to enumerate cursor positions.
    assert_eq!(
        find("hello world", "", FindOptions::permissive()),
        Vec::<Match>::new()
    );
}

#[test]
fn returns_empty_vec_when_needle_is_longer_than_haystack() {
    assert_eq!(
        find("ab", "abcdef", FindOptions::permissive()),
        Vec::<Match>::new()
    );
}

// ─────────────────────────────────────────────────────────────────
// Literal matches (case-sensitive default)
// ─────────────────────────────────────────────────────────────────

#[test]
fn finds_single_literal_match_in_middle_of_haystack() {
    // "hello world" — "world" starts at codepoint 6.
    assert_eq!(
        find("hello world", "world", FindOptions::case_sensitive()),
        vec![6..11],
    );
}

#[test]
fn finds_multiple_non_overlapping_matches() {
    // "ab ab ab" — needle "ab" at 0, 3, 6 (three matches, codepoint offsets).
    assert_eq!(
        find("ab ab ab", "ab", FindOptions::case_sensitive()),
        vec![0..2, 3..5, 6..8],
    );
}

#[test]
fn returned_matches_are_codepoint_offsets_not_byte_offsets() {
    // "café" is c-a-f-é = 4 codepoints, but 5 BYTES (é is 2 bytes in UTF-8).
    // Searching "é" must return codepoint range [3..4), not byte range [3..5).
    assert_eq!(find("café", "é", FindOptions::case_sensitive()), vec![3..4],);
}

#[test]
fn non_overlapping_aaaa_with_aa_returns_two_matches() {
    // Locked decision #4: non-overlapping. "aaaa" + "aa" → matches at
    // [0..2) and [2..4), NOT three matches at 0,1,2.
    assert_eq!(
        find("aaaa", "aa", FindOptions::case_sensitive()),
        vec![0..2, 2..4],
    );
}

#[test]
fn needle_equal_to_haystack_returns_one_full_match() {
    assert_eq!(
        find("hello", "hello", FindOptions::case_sensitive()),
        vec![0..5],
    );
}

// ─────────────────────────────────────────────────────────────────
// Case sensitivity
// ─────────────────────────────────────────────────────────────────

#[test]
fn case_insensitive_default_matches_mixed_case() {
    // Default = case-INsensitive (FindOptions::default).
    assert_eq!(
        find("Hello WORLD hello", "hello", FindOptions::default()),
        vec![0..5, 12..17],
    );
}

#[test]
fn case_sensitive_does_not_match_different_case() {
    assert_eq!(
        find("Hello WORLD", "hello", FindOptions::case_sensitive()),
        Vec::<Match>::new(),
    );
}

#[test]
fn case_sensitive_matches_only_exact_case() {
    // Sharper positive: "Hello hello Hello" with case-sensitive needle "Hello"
    // should match 0..5 and 12..17, NOT the lowercase one at 6..11.
    assert_eq!(
        find("Hello hello Hello", "Hello", FindOptions::case_sensitive()),
        vec![0..5, 12..17],
    );
}

#[test]
fn case_insensitive_matches_unicode_uppercase() {
    // Spanish ñ ↔ Ñ: case-fold should align them.
    // "Año Nuevo" with needle "ño" (lowercase) → match at codepoints 1..3
    // (A=0, ñ=1, o=2 — half-open end is 3).
    assert_eq!(find("Año Nuevo", "ÑO", FindOptions::default()), vec![1..3],);
}

// ─────────────────────────────────────────────────────────────────
// Whole-word
// ─────────────────────────────────────────────────────────────────

#[test]
fn whole_word_rejects_match_inside_a_word() {
    // "hellos" contains "hello" but is one word — whole_word must reject.
    assert_eq!(
        find("hellos", "hello", FindOptions::whole_word()),
        Vec::<Match>::new(),
    );
}

#[test]
fn whole_word_accepts_match_with_space_boundaries() {
    // "say hello world" — "hello" is bounded by spaces on both sides.
    assert_eq!(
        find("say hello world", "hello", FindOptions::whole_word()),
        vec![4..9],
    );
}

#[test]
fn whole_word_accepts_match_at_start_of_haystack() {
    // Start of haystack is always a word boundary.
    assert_eq!(
        find("hello world", "hello", FindOptions::whole_word()),
        vec![0..5],
    );
}

#[test]
fn whole_word_accepts_match_at_end_of_haystack() {
    // End of haystack is always a word boundary.
    assert_eq!(
        find("say hello", "hello", FindOptions::whole_word()),
        vec![4..9],
    );
}

#[test]
fn whole_word_treats_unicode_letter_n_tilde_as_part_of_word() {
    // Locked decision #1: ñ is alphanumeric (Unicode). So "se" inside
    // "señorita" must NOT match with whole_word — ñ is a word-internal
    // letter, not a boundary.
    assert_eq!(
        find("señorita habla", "se", FindOptions::whole_word()),
        Vec::<Match>::new(),
    );
}

#[test]
fn whole_word_with_punctuation_boundaries_matches() {
    // Punctuation (`.`, `,`, `!`) is not alphanumeric → word boundary.
    assert_eq!(
        find("hello, world!", "world", FindOptions::whole_word()),
        vec![7..12],
    );
}

#[test]
fn whole_word_underscore_is_not_a_letter() {
    // Locked decision #1: we use `is_alphanumeric()`, NOT regex `\w`.
    // Underscore IS NOT alphanumeric, so "foo" in "foo_bar" is a whole
    // word match (underscore counts as a boundary).
    assert_eq!(
        find("foo_bar", "foo", FindOptions::whole_word()),
        vec![0..3],
    );
}

#[test]
fn whole_word_combines_with_case_sensitive() {
    // Both flags on: "Hello" in "say Hello and hello" — case-sensitive
    // rejects the lowercase one, whole-word accepts the boundaried one.
    let opts = FindOptions {
        case_sensitive: true,
        whole_word: true,
    };
    assert_eq!(find("say Hello and hello", "Hello", opts), vec![4..9],);
}

// ─────────────────────────────────────────────────────────────────
// Property-based invariants (locked in module docs)
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Every returned match must land on char boundaries (we work in
    /// codepoint units, so `start`/`end` must be valid codepoint
    /// indices into the haystack).
    #[test]
    fn prop_returned_matches_land_on_codepoint_boundaries(
        haystack in "\\PC{0,100}",
        needle in "\\PC{1,10}",
    ) {
        let matches = find(&haystack, &needle, FindOptions::permissive());
        let total_chars = haystack.chars().count();
        for m in &matches {
            prop_assert!(m.start <= total_chars, "start {} out of {}", m.start, total_chars);
            prop_assert!(m.end <= total_chars, "end {} out of {}", m.end, total_chars);
            prop_assert!(m.start < m.end, "match must be non-empty: {:?}", m);
        }
    }

    /// Returned matches must be sorted ascending and non-overlapping.
    #[test]
    fn prop_returned_matches_are_sorted_and_non_overlapping(
        haystack in "\\PC{0,100}",
        needle in "\\PC{1,10}",
    ) {
        let matches = find(&haystack, &needle, FindOptions::permissive());
        for window in matches.windows(2) {
            prop_assert!(
                window[0].end <= window[1].start,
                "matches overlap: {:?} then {:?}", window[0], window[1],
            );
        }
    }
}
