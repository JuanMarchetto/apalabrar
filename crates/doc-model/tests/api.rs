//! RED-phase behaviour tests for the doc-model public API. These live in
//! `tests/` (integration target) rather than under `#[cfg(test)] mod tests`
//! in `lib.rs` because the project's pre-commit hook runs `cargo test
//! --workspace --lib --quiet`; keeping behaviour tests out of the lib
//! target lets RED commits land while still running the unit-style checks
//! on every push and in CI.
//!
//! The four assertions named in the Validation Gate 3 prompt
//! (`new_doc_is_empty`, `insert_at_zero_yields_text`,
//! `delete_range_yields_remaining_text`, `format_range_applies_mark`)
//! appear first; the rest are extras for boundary coverage.

use apalabrar_doc_model::{Doc, Mark};

// ---------- Required by Validation Gate 3 prompt ----------

#[test]
fn new_doc_is_empty() {
    let d = Doc::new();
    assert_eq!(d.text(), "");
}

#[test]
fn insert_at_zero_yields_text() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    assert_eq!(d.text(), "hello");
}

#[test]
fn delete_range_yields_remaining_text() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.delete(5..11);
    assert_eq!(d.text(), "hello");
}

#[test]
fn format_range_applies_mark() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.format(0..5, Mark::Bold);
    assert!(d.has_mark(0, Mark::Bold));
    assert!(d.has_mark(2, Mark::Bold));
    assert!(d.has_mark(4, Mark::Bold));
}

// ---------- Boundaries / extras ----------

#[test]
fn insert_at_end_appends_text() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.insert(5, " world");
    assert_eq!(d.text(), "hello world");
}

#[test]
fn delete_full_text_clears_doc() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.delete(0..5);
    assert_eq!(d.text(), "");
}

#[test]
fn format_outside_range_does_not_apply_mark() {
    let mut d = Doc::new();
    d.insert(0, "hello world");
    d.format(0..5, Mark::Bold);
    // "hello" carries the mark.
    assert!(d.has_mark(0, Mark::Bold));
    assert!(d.has_mark(4, Mark::Bold));
    // " world" does not (positions 5..=10 in a length-11 string).
    assert!(!d.has_mark(5, Mark::Bold));
    assert!(!d.has_mark(7, Mark::Bold));
    assert!(!d.has_mark(10, Mark::Bold));
}

#[test]
fn two_marks_on_same_range_both_apply() {
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.format(0..5, Mark::Bold);
    d.format(0..5, Mark::Italic);
    assert!(d.has_mark(2, Mark::Bold));
    assert!(d.has_mark(2, Mark::Italic));
}

// ---------- Snapshot / merge unit-level checks ----------

#[test]
fn snapshot_then_from_snapshot_preserves_text() {
    let mut d = Doc::new();
    d.insert(0, "hola");
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    assert_eq!(restored.text(), "hola");
}

#[test]
fn merge_empty_doc_with_full_snapshot_yields_full_text() {
    let mut donor = Doc::new();
    donor.insert(0, "hola");
    let snap = donor.snapshot();
    let mut receiver = Doc::new();
    receiver.merge(&snap);
    assert_eq!(receiver.text(), "hola");
}

// ---------- Unicode / LATAM coverage (the locked thesis) ----------

#[test]
fn multibyte_latam_text_round_trips_through_snapshot() {
    // "Año mañana es el día 1°" mixes 1- and 2-byte UTF-8 codepoints.
    // Codepoint count = 23; byte length = 27. The doc API operates on
    // codepoints so insert(0, ...) + text() must reproduce the input
    // byte-for-byte regardless of internal storage units.
    let original = "Año mañana es el día 1°";
    let mut d = Doc::new();
    d.insert(0, original);
    let snap = d.snapshot();
    let restored = Doc::from_snapshot(&snap);
    assert_eq!(restored.text(), original);
}

#[test]
fn format_at_codepoint_offset_inside_multibyte_string_marks_correct_chars() {
    // "café" — 4 codepoints (c, a, f, é). Format range 0..3 should mark
    // 'c', 'a', 'f' but NOT 'é'. The codepoint-offset spec means we
    // index by char position, not byte position.
    let mut d = Doc::new();
    d.insert(0, "café");
    d.format(0..3, Mark::Bold);
    assert!(
        d.has_mark(0, Mark::Bold),
        "'c' at codepoint 0 should be Bold"
    );
    assert!(
        d.has_mark(1, Mark::Bold),
        "'a' at codepoint 1 should be Bold"
    );
    assert!(
        d.has_mark(2, Mark::Bold),
        "'f' at codepoint 2 should be Bold"
    );
    assert!(
        !d.has_mark(3, Mark::Bold),
        "'é' at codepoint 3 should NOT be Bold"
    );
}

// ---------- Clip-behavior contract (documented in lib.rs) ----------

#[test]
fn insert_past_doc_length_clips_to_end_without_panic() {
    let mut d = Doc::new();
    d.insert(0, "hi");
    d.insert(999, "!"); // far past length 2 — must clip, not panic.
    assert_eq!(d.text(), "hi!");
}

#[test]
fn delete_past_doc_length_clips_to_end_without_panic() {
    let mut d = Doc::new();
    d.insert(0, "hi");
    d.delete(0..999); // far past end — must clip, not panic.
    assert_eq!(d.text(), "");
}

#[test]
fn format_past_doc_length_clips_range_without_panic() {
    let mut d = Doc::new();
    d.insert(0, "hi");
    d.format(0..999, Mark::Bold); // clip to 0..2 internally.
    assert!(d.has_mark(0, Mark::Bold));
    assert!(d.has_mark(1, Mark::Bold));
}

#[test]
fn format_with_empty_range_is_noop() {
    // Empty range (start == end) must take the no-op short-circuit and
    // not call into Loro's mark API, where an empty range would error.
    let mut d = Doc::new();
    d.insert(0, "hello");
    d.format(2..2, Mark::Bold);
    assert!(!d.has_mark(2, Mark::Bold));
    assert!(!d.has_mark(0, Mark::Bold));
}

#[test]
fn format_inverted_range_clips_to_empty_and_is_noop() {
    // start > end is a degenerate input. apply_op clips it; format must
    // accept the resulting empty range without panic. Variables avoid
    // the compile-time-empty-range clippy lint while still exercising
    // the clip path.
    let mut d = Doc::new();
    d.insert(0, "hello");
    let start: usize = 4;
    let end: usize = 1;
    d.format(start..end, Mark::Bold);
    assert!(!d.has_mark(2, Mark::Bold));
}

// ---------- has_mark fallthrough + Default ----------

#[test]
fn has_mark_past_doc_length_returns_false() {
    let mut d = Doc::new();
    d.insert(0, "hi");
    d.format(0..2, Mark::Bold);
    // pos == len: cursor position past last codepoint, not a character.
    assert!(!d.has_mark(2, Mark::Bold));
    // way past — same answer.
    assert!(!d.has_mark(999, Mark::Bold));
}

#[test]
fn has_mark_on_empty_doc_returns_false() {
    let d = Doc::new();
    assert!(!d.has_mark(0, Mark::Bold));
}

#[test]
fn default_constructs_empty_doc_equivalent_to_new() {
    let d: Doc = Default::default();
    assert_eq!(d.text(), "");
    assert!(!d.has_mark(0, Mark::Bold));
}
