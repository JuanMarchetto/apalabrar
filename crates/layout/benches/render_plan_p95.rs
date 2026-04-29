//! Phase 4.1 acceptance bench — `layout(doc, vp)` over a 200-page
//! synthetic doc, reporting p50/p95/p99 across 1000 individual calls.
//! Exits non-zero if p95 exceeds the 50 ms gate.
//!
//! Run with: `cargo bench -p apalabrar-layout --bench render_plan_p95`.
//!
//! Differs from `keystroke_p95` in two ways:
//!   - it consumes `apalabrar_doc_model::Doc` (canonical CRDT-backed
//!     model) rather than the legacy `Document` shape, exercising the
//!     full Phase 4.1 pipeline (doc-model projection → layout → plan);
//!   - the budget is 50 ms (Prompt 4.1 acceptance) rather than 16 ms
//!     (Gate 5 keystroke budget). 200-page docs are bigger than the
//!     10 K-block keystroke corpus so the looser budget reflects the
//!     larger workload, not a relaxation of the keystroke target.

use apalabrar_doc_model::Doc;
use apalabrar_layout::{LETTER_AT_96DPI, layout};
use std::time::Instant;

const N_CALLS: usize = 1000;
const P95_BUDGET_MS: f64 = 50.0;

/// Build a doc large enough to need ~200 pages on Letter @ 96 DPI.
/// Each paragraph is ~80 chars (one line at 14 px font on a 624 px
/// content area). Letter content height is 864 px; with paragraph
/// height ~26 px (line + 8 px after) we fit ~33 paragraphs per page,
/// so 200 pages need ~6600 paragraphs.
fn synthetic_doc(n_blocks: usize) -> Doc {
    let mut texts: Vec<String> = Vec::with_capacity(n_blocks);
    for i in 0..n_blocks {
        match i % 10 {
            0 => texts.push(format!(
                "Section {i} — Synthetic heading for layout benchmarking"
            )),
            1 | 2 => texts.push(format!(
                "List item {i}: bullet content with deterministic text body so shaping reproduces"
            )),
            _ => texts.push(format!(
                "Paragraph {i}. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do."
            )),
        }
    }
    let mut d = Doc::new();
    d.insert(0, &texts.join("\n"));
    d
}

fn percentile(sorted_ms: &[f64], p: f64) -> f64 {
    let idx = ((sorted_ms.len() as f64 - 1.0) * p).round() as usize;
    sorted_ms[idx]
}

fn main() {
    let doc = synthetic_doc(6_600);

    // 100-call warmup so allocator state and branch predictors settle.
    for _ in 0..100 {
        let _ = layout(&doc, &LETTER_AT_96DPI).expect("layout succeeds");
    }

    let mut samples_ms: Vec<f64> = Vec::with_capacity(N_CALLS);
    for _ in 0..N_CALLS {
        let t0 = Instant::now();
        let plan = layout(&doc, &LETTER_AT_96DPI).expect("layout succeeds");
        let dt = t0.elapsed();
        // Touch a field so the optimiser cannot drop the call entirely.
        std::hint::black_box(plan.page_count());
        samples_ms.push(dt.as_secs_f64() * 1000.0);
    }
    samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = percentile(&samples_ms, 0.50);
    let p95 = percentile(&samples_ms, 0.95);
    let p99 = percentile(&samples_ms, 0.99);
    let min = samples_ms.first().copied().unwrap();
    let max = samples_ms.last().copied().unwrap();
    let mean = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;

    println!("# Phase 4.1 acceptance — render_plan p95 over 200-page doc");
    println!("# corpus: ~6 600 mixed blocks (paragraph / heading / list)");
    println!("# operation: layout(&doc, &LETTER_AT_96DPI)");
    println!("# samples: {N_CALLS}");
    println!("min       = {:>8.3} ms", min);
    println!("p50       = {:>8.3} ms", p50);
    println!("mean      = {:>8.3} ms", mean);
    println!("p95       = {:>8.3} ms", p95);
    println!("p99       = {:>8.3} ms", p99);
    println!("max       = {:>8.3} ms", max);
    println!(
        "budget    = {:>8.3} ms (Prompt 4.1: p95 ≤ 50 ms)",
        P95_BUDGET_MS
    );
    if p95 > P95_BUDGET_MS {
        eprintln!("\n[FAIL] p95 {p95:.3} ms exceeds gate budget {P95_BUDGET_MS} ms");
        std::process::exit(1);
    }
    let margin_factor = P95_BUDGET_MS / p95;
    println!(
        "\n[PASS] p95 {p95:.3} ms is {margin_factor:.1}× under the {P95_BUDGET_MS} ms gate budget"
    );
}
