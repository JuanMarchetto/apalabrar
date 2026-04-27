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
