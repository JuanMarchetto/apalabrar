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

/// Sample marks at every codepoint in a range. Helper for asserting mark
/// equality across replicas.
fn marks_along(doc: &Doc, range: Range<usize>) -> Vec<(usize, bool, bool)> {
    range
        .map(|pos| {
            (
                pos,
                doc.has_mark(pos, Mark::Bold),
                doc.has_mark(pos, Mark::Italic),
            )
        })
        .collect()
}

proptest! {
    // 256 cases is the audit-time setting for fast iteration; the gate run
    // (per the Validation Gate 3 prompt) bumps `cases` to 10_000. We also
    // raise `max_local_rejects` so the rich-text mark prop, which uses
    // `prop_assume!` to filter to overlapping ranges, doesn't fail
    // spuriously on rejection saturation at scale.
    #![proptest_config(ProptestConfig {
        cases: 256,
        max_local_rejects: 100_000,
        ..ProptestConfig::default()
    })]

    /// Convergence: two replicas that apply disjoint subsets of the same
    /// op set and then exchange snapshots converge to equal text AND
    /// equal marks. Marks parity is sampled across the full text range.
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
        let len = a.text().chars().count();
        prop_assert_eq!(marks_along(&a, 0..len), marks_along(&b, 0..len));
    }

    /// Idempotence: applying the same snapshot twice yields the same text
    /// AND the same marks as applying it once.
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
        let len = once.text().chars().count();
        prop_assert_eq!(marks_along(&once, 0..len), marks_along(&twice, 0..len));
    }

    /// Causal preservation across a 3-replica chain.
    /// A applies an "anchor" insert at offset 0, exports.
    /// B imports A, then applies a "follow-up" insert at the end of the
    /// anchor, exports.
    /// C imports B's snapshot — which transitively carries A's history.
    /// C must observe both A's text and B's text in the order A→B; the
    /// follow-up cannot appear without the anchor it depends on.
    #[test]
    fn prop_three_replica_causal_chain_preserves_order(
        anchor in "[a-z]{1,5}",
        follow in "[a-z]{1,5}",
    ) {
        let mut a = Doc::new();
        a.insert(0, &anchor);
        let snap_a = a.snapshot();

        let mut b = Doc::from_snapshot(&snap_a);
        // Append at end of A's text — depends on A having been merged.
        let anchor_len = anchor.chars().count();
        b.insert(anchor_len, &follow);
        let snap_b = b.snapshot();

        let c = Doc::from_snapshot(&snap_b);
        let expected = format!("{anchor}{follow}");
        prop_assert_eq!(c.text(), expected.clone(),
            "C must see anchor+follow in causal order; got {:?} expected {:?}",
            c.text(), expected);
    }

    /// Round-trip: a doc reconstructed from its own snapshot has the
    /// same text AND marks as the original. Stronger than the unit
    /// version because the op sequence (and thus the mark layout) is
    /// randomised.
    #[test]
    fn prop_snapshot_round_trip_preserves_text_and_marks(
        ops in proptest::collection::vec(any_op(), 0..15),
    ) {
        let mut original = Doc::new();
        for op in &ops {
            apply_test_op(&mut original, op);
        }
        let snap = original.snapshot();
        let restored = Doc::from_snapshot(&snap);
        prop_assert_eq!(restored.text(), original.text());
        let len = original.text().chars().count();
        prop_assert_eq!(marks_along(&original, 0..len), marks_along(&restored, 0..len));
    }

    /// Rich-text mark concurrency: two replicas fork from a shared
    /// ancestor, apply Bold and Italic respectively over overlapping
    /// ranges, then merge. Both marks must be present on every
    /// codepoint in the intersection. Additionally, codepoints outside
    /// both ranges (the unmarked complement) must carry NEITHER mark —
    /// guards against a Loro impl that over-applies marks during merge.
    ///
    /// Strategy design: ranges are anchored on a shared `center`
    /// codepoint and constructed so that BOTH always contain `center`.
    /// This guarantees a non-empty intersection by construction, so
    /// the test needs no `prop_assume!` and never spills global
    /// rejection budget at high case counts (the audit-phase concern
    /// that surfaced at first GREEN run as "too many global rejects").
    #[test]
    fn prop_concurrent_marks_both_apply_on_intersection(
        seed_text in "[a-z]{4,30}",
        center_choice in 0usize..30,
        a_left in 0usize..10,
        a_right in 1usize..10,
        b_left in 0usize..10,
        b_right in 1usize..10,
    ) {
        let mut base = Doc::new();
        base.insert(0, &seed_text);
        let base_len = seed_text.chars().count();
        // base_len >= 4 by strategy regex.
        let center = center_choice.min(base_len - 1);

        let a_s = center.saturating_sub(a_left);
        let a_e = (center + a_right).min(base_len);
        let b_s = center.saturating_sub(b_left);
        let b_e = (center + b_right).min(base_len);
        // Invariant: a_s <= center < a_e and b_s <= center < b_e, so
        // both ranges include `center` and the intersection contains
        // at least that codepoint.

        let snap = base.snapshot();
        let mut a = Doc::from_snapshot(&snap);
        let mut b = Doc::from_snapshot(&snap);

        a.format(a_s..a_e, Mark::Bold);
        b.format(b_s..b_e, Mark::Italic);
        a.merge(&b.snapshot());

        let inter_s = a_s.max(b_s);
        let inter_e = a_e.min(b_e);
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

        // Negative side: positions outside the union of both ranges
        // must carry neither mark. Guards against marks "leaking" past
        // their intended span during merge.
        let union_s = a_s.min(b_s);
        let union_e = a_e.max(b_e);
        for pos in 0..base_len {
            if pos >= union_s && pos < union_e {
                continue;
            }
            prop_assert!(!a.has_mark(pos, Mark::Bold),
                "Bold leaked to pos {pos} outside union [{union_s},{union_e})");
            prop_assert!(!a.has_mark(pos, Mark::Italic),
                "Italic leaked to pos {pos} outside union [{union_s},{union_e})");
        }
    }
}
