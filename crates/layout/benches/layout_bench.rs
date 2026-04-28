//! Validation Gate 5 — keystroke-to-render p95 bench (Rust-only stage).
//!
//! Two scenarios:
//!
//! - `layout_10k_cold` — full layout from scratch on a 10 000-block synthetic
//!   document (mixed paragraph + heading + list). Establishes the upper bound
//!   for first-paint after document load.
//! - `layout_10k_incremental_mid` — `relayout_after_change` at the middle of
//!   the same document. This is the keystroke-equivalent path: re-shape one
//!   block, re-pack pages over the cache. Gate criterion: incremental p95 < 16 ms.
//!
//! The Playwright keystroke-to-render bench (CPU-throttled, full editor wired
//! through CRDT + paint) is deferred to a follow-up gate after the editor is
//! wired into the WASM bridge.

use apalabrar_layout::{Block, Document, Engine, LETTER_AT_96DPI};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Generate a deterministic 10 000-block synthetic document. Mix:
///
/// - 70 % body paragraphs
/// - 10 % headings (levels 1–3)
/// - 20 % bulleted list items (indent 0–2)
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

fn bench_layout_10k_cold(c: &mut Criterion) {
    let doc = synthetic_doc(10_000);
    let mut group = c.benchmark_group("layout_10k_cold");
    group.sample_size(10);
    group.bench_function("full_layout", |b| {
        b.iter_with_large_drop(|| {
            let mut engine = Engine::new(LETTER_AT_96DPI).expect("font loads");
            engine.layout(black_box(&doc))
        });
    });
    group.finish();
}

fn bench_layout_10k_incremental_mid(c: &mut Criterion) {
    let doc = synthetic_doc(10_000);
    let mid = doc.len() / 2;

    let mut engine = Engine::new(LETTER_AT_96DPI).expect("font loads");
    let _ = engine.layout(&doc);

    let mut group = c.benchmark_group("layout_10k_incremental");
    group.sample_size(50);
    group.bench_function("relayout_middle", |b| {
        b.iter(|| {
            engine
                .relayout_after_change(black_box(&doc), black_box(mid))
                .expect("valid index")
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_layout_10k_cold,
    bench_layout_10k_incremental_mid
);
criterion_main!(benches);
