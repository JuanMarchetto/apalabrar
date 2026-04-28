//! Per-call latency distribution for `relayout_after_change` — reports
//! p50/p95/p99 across 1000 individual calls. Criterion's batched samples
//! converge on the mean; this harness gives the actual percentiles needed
//! to gate on the keystroke-to-render p95 < 16 ms criterion.
//!
//! Run with `cargo bench -p apalabrar-layout --bench keystroke_p95`.
//!
//! This is an instrumentation-only bench (no Criterion harness) so output
//! is plain stdout. Exit code is non-zero if p95 exceeds the budget.

use apalabrar_layout::{Block, Document, Engine, LETTER_AT_96DPI};
use std::time::Instant;

const N_CALLS: usize = 1000;
const P95_BUDGET_MS: f64 = 16.0;

fn synthetic_doc(n: usize) -> Document {
    let mut blocks = Vec::with_capacity(n);
    for i in 0..n {
        match i % 10 {
            0 => blocks.push(Block::Heading {
                level: ((i / 10) % 3 + 1) as u8,
                text: format!("Section {i} — Synthetic heading for layout benchmarking"),
            }),
            1 | 2 => blocks.push(Block::ListItem {
                indent: (i % 3) as u8,
                text: format!(
                    "List item {i}: bullet content with deterministic text body so shaping is reproducible across runs"
                ),
            }),
            _ => blocks.push(Block::Paragraph {
                text: format!(
                    "Paragraph {i}. Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim \
                     ad minim veniam, quis nostrud exercitation ullamco laboris."
                ),
            }),
        }
    }
    Document::new(blocks)
}

fn percentile(sorted_ms: &[f64], p: f64) -> f64 {
    let idx = ((sorted_ms.len() as f64 - 1.0) * p).round() as usize;
    sorted_ms[idx]
}

fn main() {
    let doc = synthetic_doc(10_000);
    let mid = doc.len() / 2;

    let mut engine = Engine::new(LETTER_AT_96DPI).expect("font loads");
    let _ = engine.layout(&doc);

    // 100-call warmup so JIT-like effects (allocator state, branch predictors)
    // settle before the measured run.
    for _ in 0..100 {
        let _ = engine
            .relayout_after_change(&doc, mid)
            .expect("valid index");
    }

    let mut samples_ms: Vec<f64> = Vec::with_capacity(N_CALLS);
    for _ in 0..N_CALLS {
        let t0 = Instant::now();
        let _result = engine
            .relayout_after_change(&doc, mid)
            .expect("valid index");
        let dt = t0.elapsed();
        samples_ms.push(dt.as_secs_f64() * 1000.0);
    }
    samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = percentile(&samples_ms, 0.50);
    let p95 = percentile(&samples_ms, 0.95);
    let p99 = percentile(&samples_ms, 0.99);
    let min = samples_ms.first().copied().unwrap();
    let max = samples_ms.last().copied().unwrap();
    let mean = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;

    println!("# Apalabrar Validation Gate 5 — Rust-only relayout latency");
    println!("# corpus: 10 000 mixed blocks (paragraph/heading/list)");
    println!("# operation: relayout_after_change at the middle block");
    println!("# samples: {N_CALLS}");
    println!("min       = {:>8.3} ms", min);
    println!("p50       = {:>8.3} ms", p50);
    println!("mean      = {:>8.3} ms", mean);
    println!("p95       = {:>8.3} ms", p95);
    println!("p99       = {:>8.3} ms", p99);
    println!("max       = {:>8.3} ms", max);
    println!(
        "budget    = {:>8.3} ms (criterion: incremental p95 < 16 ms)",
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
