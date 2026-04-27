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
