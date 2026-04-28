# Changelog

All notable changes to Apalabrar are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- Initial repository skeleton — Cargo workspace (11 crates), pnpm workspace
  (4 packages), Turborepo task graph, lefthook pre-commit hooks, Conventional
  Commits gate.
- TDD scaffolding — every crate has `tests/`, `benches/`, and a stub unit test.
- CI pipeline — typecheck, lint, format, unit, integration, E2E (Chromium),
  coverage gate, bundle budget, mutation diff, accessibility, license audit.
- Nightly pipeline — full mutation, multi-browser E2E, full corpus,
  performance benchmarks, dependency audit.
- ATTRIBUTION.md, CONTRIBUTING.md (with Founder Discipline Pact),
  CODE_OF_CONDUCT.md (Contributor Covenant 2.1), SECURITY.md.
- Validation Gate 1 (LATAM dead-key composition):
  `applyComposition(prev, evt)` for ñ/á/é/í/ó/ú/ü/ç/ã with NFC normalization;
  `ComposingEditor` Solid component wired to compositionstart/end + beforeInput
  controlled-input pattern; route at `/composer` mounts the editor; 21 vitest
  tests (15 dead-key spec + 5 ComposingEditor wiring + 1 fast-check property
  invariant) and 10 Playwright tests across Chromium/Firefox/WebKit.
- Validation Gate 5 — Rust-only stage (keystroke-to-render p95 GO/NO-GO,
  Path B perf): `apalabrar-layout` ships a paged layout engine
  (`Engine::{new, layout, relayout_after_change}`) over a flat `Document`
  / `Block` model (Paragraph, Heading{level}, ListItem{indent}). The
  engine bundles DejaVu Sans regular (744 KB, DejaVu license, embedded
  via `include_bytes!`) so shaping is reproducible across machines, and
  drives shaping + line-breaking through `cosmic-text 0.12`'s
  `Buffer::set_text → shape_until_scroll → layout_runs` pipeline. The
  cache is a `BTreeMap<usize, ShapedBlock>` keyed by block index;
  `relayout_after_change` re-shapes only the changed block before
  re-packing pages. Length mismatches transparently fall back to a full
  layout. 37 tests across the crate pass GREEN: 2 lib (version pin +
  bundled-font sanity), 26 behavioural in `tests/api.rs` (construction,
  viewport math, empty doc, single block, pagination, kind round-trips,
  heading-vs-paragraph height, indent → origin_x and wrap-width math,
  LATAM accents, determinism, cache invariants, in-place relayout ≡
  fresh layout, OOB index, length-change fallback, line-sum bound, long
  paragraph wrap), 6 property tests in `tests/properties.rs` (block-box
  count == doc.len, layout determinism, every block_index exactly once,
  no block taller than a page, OOB always errors, in-place relayout
  matches fresh), and 3 insta YAML snapshots reviewed manually before
  `cargo insta accept`. The new `keystroke_p95` bench measures 1 000
  individual `relayout_after_change` calls at the document midpoint on
  a 10 000-block synthetic corpus (mixed paragraph/heading/list) and
  reports p50 = 2.09 ms, p95 = **3.79 ms**, p99 = 4.89 ms, max = 7.42 ms
  — **4.2× under the 16 ms gate budget**. The Criterion `layout_bench`
  reports cold full-layout 1.24-1.59-1.92 s and a sample-mean
  incremental band of 1.85-2.00 ms over 50 batched samples.
  cargo-mutants kills 59/61 (96.7 %) — well past the 85 % floor for
  non-moat crates; the two surviving mutations are float-arithmetic
  perturbations on the page-overflow check (`>` vs `>=` and the sign of
  `f32::EPSILON`) that are inherently equivalent on non-edge inputs.
  `cargo llvm-cov` reports 97.42 % region / 100 % function / 94.30 %
  line on `apalabrar-layout/src/lib.rs`. The Playwright keystroke-to-
  render bench (CPU-throttled, full editor wired through CRDT + paint)
  is deferred to a follow-up gate after the editor is wired into the
  WASM bridge — this stage validates only the layout half of the
  keystroke chain. Phase 1 closure remains pending the Playwright run.
- Validation Gate 4 (OOXML round-trip moat GO/NO-GO): structural surface
  in `apalabrar_format_docx` (`read(&[u8]) -> Result<DocModel, Error>`,
  `write(&DocModel) -> Result<Vec<u8>, Error>`, plus `paragraph_count`,
  `paragraph_text`, `set_paragraph_text` on `DocModel`) implementing the
  shadow-store pattern: zip parts kept verbatim, `word/document.xml`
  indexed once for paragraph-level edits via `quick-xml`, dirty
  paragraphs spliced back into a fresh `document.xml` while every
  byte-range around them and every other zip part round-trips
  byte-equivalent (the lossless contract). Six fixtures committed under
  `tests-corpus/` (5 LibreOffice-produced — academic / multilingual /
  tables / footnotes — plus 1 synthetic `<w:p/>` self-closing edge case)
  pass byte-equivalent round-trip after whitespace + attribute-order
  normalization. 33 tests across the format-docx test surface (1 lib +
  9 flat-text Gate-2 carry-over + 2 fuzz-companion properties + 16
  structural round-trip + 5 insta YAML snapshots, all reviewed manually
  before `cargo insta accept`). cargo-fuzz `docx_read` target ran
  2 425 032 iterations in 601 s with 0 crashes / panics / OOMs / hangs
  and 1775 covered features. cargo-mutants killed 34 / 36 mutants
  (94.4 % — the two surviving mutations are inherently equivalent on
  conformant OOXML and would require malformed synthetic input or
  removal of defensive code to flip; documented as harmless). cargo
  llvm-cov reports 91.21 % region / 94.39 % line coverage on
  `format-docx::lib.rs` (function coverage understated by `thiserror`-
  generated `Display` impls).
- Validation Gate 2 (WASM bundle size GO/NO-GO for Path B):
  `apalabrar-format-docx::{parse_text, serialize_text}` (docx-rs round-trip
  through plain UTF-8 with `\n` paragraph separators) and
  `apalabrar-editor-core::{open_docx, apply_op, to_docx, doc_text,
  close_doc}` over a static `OnceLock<Mutex<HashMap<u64, Doc>>>` registry,
  bridging UTF-8 byte offsets at the JS boundary to Loro's codepoint
  offsets internally; wasm-bindgen exports the same surface to
  JavaScript via a `cfg(target_arch = "wasm32")` submodule, gated so
  host-side `cargo test` stays free of JsValue. `wasm-pack build --target
  web --release` compiles to **2.78 MB raw / 1.10 MB gzipped (813 KB
  brotli)** — well below the 4 MB target / 10 MB hard ceiling. The
  Solid app gains a `/demo` route in `packages/app/src/pages/Demo.tsx`
  that calls `init() → openDocx → docText` over a committed
  `tests-corpus/demo/sample.docx` (18 KB, generated by a one-off cargo
  example using format-docx), reached via Vite's `?url` import after
  opening `server.fs.allow` to the workspace root and adding `*.docx`
  to `assetsInclude`. `@apalabrar/editor-bridge` is a thin pnpm
  `file:`-dep re-export of the `wasm-pack` pkg. The
  `packages/app/package.json` `size-limit` block now caps the
  brotli-compressed JS bundle at 200 KB and the WASM core at 4 MB;
  the current measurement is 6.61 KB JS and 813 KB WASM. 47 tests
  total across the cascade pass GREEN: 1 + 27 in editor-core, 1 + 9
  in format-docx, 1 + 25 in doc-model.
- Validation Gate 3 (CRDT layer GO/NO-GO): `apalabrar-doc-model` exposes
  `Doc::{new, insert, delete, format, snapshot, merge, from_snapshot, text,
  has_mark, default}` over Unicode codepoint offsets, backed by `loro 1.12`
  (single text container "doc" with rich-text marks "bold"/"italic" using
  default expand-after semantics); 26 tests total (1 lib version pin, 20
  behaviour assertions in `tests/api.rs` covering happy paths, edges,
  multibyte LATAM, clip-defensive contract, error fallthroughs, and
  `Default`, plus 5 property tests in `tests/properties.rs` — convergence
  with mark parity, idempotence with mark parity, 3-replica causal chain,
  snapshot round-trip preserving text+marks, and concurrent Bold+Italic
  marks composing on intersection with a negative leak guard outside the
  union). Gate run at `PROPTEST_CASES=10000` exercises 50 000 random
  CRDT scenarios in 6.6 s release; cargo-mutants kills 26/26 mutants
  (100 %); cargo-llvm-cov reports 99.35 % region / 98.88 % line / 100 %
  function coverage (one defensive `else { continue; }` arm in
  `has_mark` is dead per Loro's `to_delta` contract and remains
  uncovered intentionally).

### Changed

_n/a — initial release_

### Deprecated

_n/a_

### Removed

_n/a_

### Fixed

_n/a_

### Security

_n/a_

---

[Unreleased]: https://github.com/apalabrar/apalabrar/compare/HEAD...HEAD
