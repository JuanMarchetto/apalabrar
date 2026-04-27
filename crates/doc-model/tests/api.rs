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
