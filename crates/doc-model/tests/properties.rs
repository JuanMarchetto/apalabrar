//! RED-phase property tests for doc-model (Loro CRDT layer).
//!
//! These five properties encode the CRDT invariants Apalabrar relies on
//! and drive Validation Gate 3:
//!
//! 1. Convergence — replicas with the same set of ops converge.
//! 2. Idempotence — `merge(snapshot)` twice == once.
//! 3. Causal preservation — local op order is preserved through snapshot.
//! 4. Round-trip — `from_snapshot(doc.snapshot())` has the same text.
//! 5. Rich-text mark concurrency — concurrent Bold + Italic over
//!    overlapping ranges leaves both marks on the intersection.
//!
//! Tests are RED until Step 4 wires the Loro implementation.
//! `cases: 256` is the audit-time setting; the gate run bumps to 10_000.

use apalabrar_doc_model::{Doc, Mark};
use proptest::prelude::*;
use std::ops::Range;

#[derive(Debug, Clone)]
enum TestOp {
    Insert(usize, String),
    Delete(Range<usize>),
    Format(Range<usize>, Mark),
}

fn any_op() -> impl Strategy<Value = TestOp> {
    prop_oneof![
        (0usize..30, "[a-z ]{0,5}").prop_map(|(p, t)| TestOp::Insert(p, t)),
        (0usize..30, 0usize..30).prop_map(|(s, e)| TestOp::Delete(s..e)),
        (
            0usize..30,
            0usize..30,
            prop_oneof![Just(Mark::Bold), Just(Mark::Italic)],
        )
            .prop_map(|(s, e, m)| TestOp::Format(s..e, m)),
    ]
}

/// Apply a generated op against a doc, clipping into bounds. The Doc API
/// guarantees clipping for insert/delete/format, but the helper centralises
/// the bound discipline so each test reads as logic-only.
fn apply_test_op(doc: &mut Doc, op: &TestOp) {
    let len = doc.text().chars().count();
    match op {
        TestOp::Insert(pos, text) => {
            let pos = (*pos).min(len);
            doc.insert(pos, text);
        }
        TestOp::Delete(range) => {
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            doc.delete(start..end);
        }
        TestOp::Format(range, mark) => {
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            if start < end {
                doc.format(start..end, *mark);
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Convergence: two replicas that apply disjoint subsets of the same
    /// op set and then exchange snapshots converge to equal text.
    #[test]
    fn prop_two_replicas_converge_after_snapshot_exchange(
        ops in proptest::collection::vec(any_op(), 0..15),
        split in 0usize..15,
    ) {
        let mut a = Doc::new();
        let mut b = Doc::new();
        let split = split.min(ops.len());
        for op in &ops[..split] {
            apply_test_op(&mut a, op);
        }
        for op in &ops[split..] {
            apply_test_op(&mut b, op);
        }
        let snap_a = a.snapshot();
        let snap_b = b.snapshot();
        a.merge(&snap_b);
        b.merge(&snap_a);
        prop_assert_eq!(a.text(), b.text());
    }

    /// Idempotence: applying the same snapshot twice yields the same text
    /// as applying it once. Both execution orders must converge to the
    /// same observable state.
    #[test]
    fn prop_merge_is_idempotent(
        ops in proptest::collection::vec(any_op(), 0..10),
    ) {
        let mut donor = Doc::new();
        for op in &ops {
            apply_test_op(&mut donor, op);
        }
        let snap = donor.snapshot();

        let mut once = Doc::new();
        once.merge(&snap);

        let mut twice = Doc::new();
        twice.merge(&snap);
        twice.merge(&snap);

        prop_assert_eq!(once.text(), twice.text());
    }

    /// Causal preservation: ops applied in order on a single replica are
    /// faithfully reflected in the snapshot. A fresh replica created from
    /// that snapshot ends up with the same observable text — i.e. local
    /// causal order is not lost across the snapshot boundary.
    #[test]
    fn prop_causal_order_preserved_through_snapshot(
        ops in proptest::collection::vec(any_op(), 1..10),
    ) {
        let mut origin = Doc::new();
        for op in &ops {
            apply_test_op(&mut origin, op);
        }
        let target_text = origin.text();
        let snap = origin.snapshot();
        let receiver = Doc::from_snapshot(&snap);
        prop_assert_eq!(receiver.text(), target_text);
    }

    /// Round-trip: a doc reconstructed from its own snapshot has the
    /// same observable text as the original. (Stronger than a single-op
    /// equivalent because the op sequence is randomised.)
    #[test]
    fn prop_snapshot_round_trip_preserves_text(
        ops in proptest::collection::vec(any_op(), 0..15),
    ) {
        let mut original = Doc::new();
        for op in &ops {
            apply_test_op(&mut original, op);
        }
        let snap = original.snapshot();
        let restored = Doc::from_snapshot(&snap);
        prop_assert_eq!(restored.text(), original.text());
    }

    /// Rich-text mark concurrency: two replicas that fork from a shared
    /// ancestor, apply Bold and Italic respectively over (possibly
    /// overlapping) ranges, and then merge — both marks must be present
    /// on every codepoint in the intersection of the two ranges.
    #[test]
    fn prop_concurrent_marks_both_apply_on_intersection(
        seed_text in "[a-z]{4,30}",
        a_start in 0usize..30,
        a_len in 1usize..15,
        b_start in 0usize..30,
        b_len in 1usize..15,
    ) {
        let mut base = Doc::new();
        base.insert(0, &seed_text);
        let base_len = seed_text.chars().count();
        let snap = base.snapshot();
        let mut a = Doc::from_snapshot(&snap);
        let mut b = Doc::from_snapshot(&snap);

        let a_e = (a_start + a_len).min(base_len);
        let a_s = a_start.min(a_e);
        let b_e = (b_start + b_len).min(base_len);
        let b_s = b_start.min(b_e);

        prop_assume!(a_s < a_e && b_s < b_e);
        let inter_s = a_s.max(b_s);
        let inter_e = a_e.min(b_e);
        prop_assume!(inter_s < inter_e);

        a.format(a_s..a_e, Mark::Bold);
        b.format(b_s..b_e, Mark::Italic);
        a.merge(&b.snapshot());

        for pos in inter_s..inter_e {
            prop_assert!(
                a.has_mark(pos, Mark::Bold),
                "expected Bold at pos {pos}; ranges a=[{a_s},{a_e}) b=[{b_s},{b_e})"
            );
            prop_assert!(
                a.has_mark(pos, Mark::Italic),
                "expected Italic at pos {pos}; ranges a=[{a_s},{a_e}) b=[{b_s},{b_e})"
            );
        }
    }
}
