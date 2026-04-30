# Contributing to Apalabrar

Thank you for considering a contribution. This project ships under furious TDD discipline — that is
non-negotiable, and this document explains exactly what that means in practice.

If you've never read `tdd-execution-plan.md`, the short version is below. The long version is the
canonical source of truth.

---

## The rules you will not break

### Rule 1 — Test-first, absolute

No line of implementation code is written until tests covering its behavior exist and **fail**. The
natural state of new code is RED. RED is good. RED means we know what we're building.

### Rule 2 — A million failing tests is fine

When starting a new module, write **all** tests for the module's intended behavior first. Run the
suite. Watch most of them fail. Track "% passing" as a KPI that climbs as implementation progresses.

### Rule 3 — Tests are immutable except when they're wrong

A test never changes to make it pass. A test changes only when:

1. The test was incorrect (asserting the wrong invariant, testing implementation detail by accident,
   testing a constant).
2. The behavior of the system **intentionally** changed and the test's assertion no longer reflects
   the spec.

PRs that modify tests must explicitly justify (in the commit message) which of the two cases
applies.

### Rule 4 — Tests must be correct

A test that always passes is worse than no test. We validate correctness via:

- **Mutation testing** (`cargo-mutants`) — surviving mutations = inadequate tests.
- **Property-based assertions** — invariants that hold for all inputs.
- **Differential testing** against reference implementations (Yjs for CRDT, citeproc-js for
  citations, MS Word for OOXML).
- **PR review checklist** (below).
- **Self-audit checklist** for vacuous tests, missing edge cases, and assertion completeness.

### Rule 5 — Coverage 100% is the guideline; mutation kill rate is the floor

| Layer                    | Line cov | Branch cov | Mutation kill | Notes              |
| ------------------------ | -------- | ---------- | ------------- | ------------------ |
| Doc model (Loro wrapper) | 100%     | 95%        | 95%           | + proptest         |
| Rope (jumprope wrapper)  | 100%     | 100%       | 95%           |                    |
| Layout engine            | 95%      | 90%        | 90%           | + insta snapshots  |
| OOXML I/O                | 100%     | 100%       | 95%           | **the moat**       |
| Citation engine          | 100%     | 100%       | 95%           |                    |
| Plugin host              | 100%     | 100%       | 95%           | security boundary  |
| JS↔Rust bridge           | 100%     | 100%       | n/a           | small surface      |
| Solid UI components      | 85%      | 75%        | n/a           | covered by E2E     |
| Sync server              | 100%     | 100%       | 95%           | revenue + security |
| **Overall floor**        | **≥90%** | **≥85%**   | **≥85%**      |                    |

These are PR-merge blocks. CI fails if violated.

### Rule 6 — Tests get pre-merge weight equal to feature code

A PR is incomplete if its tests are weak, regardless of whether the feature "works". Reviewers ask
"do these tests sufficiently cover the change?" **before** "does this change look right?".

---

## The 5-step rhythm for every feature

```
1. SPEC      Write the contract (Rust trait, TS interface, CSL spec subset).
             Comment-driven. No implementation.
2. TESTS     Write all tests. Watch them fail. Track count.
3. AUDIT     Self-review for vacuous tests. Run cargo-mutants on the stub.
4. IMPLEMENT Minimum code to make ONE test pass. Commit. Repeat.
5. POLISH    Refactor with safety net. Run mutation. Add tests for survivors.
             Update CHANGELOG. PR with coverage diff + mutation report.
```

---

## Anti-patterns we explicitly reject

### A. The Tautology

```rust
#[test]
fn test_min_page_is_one() {
    assert_eq!(MIN_PAGE, 1); // testing a constant, never fails
}
```

### B. The Mock-of-Self

Mocks belong only at the OUTSIDE boundary (filesystem, network, time). Don't mock the system under
test.

### C. The Aspirational Comment

A test without an assertion is not a test. Either fully written or doesn't exist.

### D. The Implementation Mirror

Test observable behavior, not internal field values. `assert_eq!(result.internal_field_x, 42)` is
brittle to refactoring.

### E. The Order-Dependent

Tests must be independently runnable. Use fresh state per test. No shared mutable globals.

### F. The Smoke Test Masquerading as Unit Test

If your unit test exercises four modules, it's an integration test. Split it.

---

## When TDD is **not** required (the six exceptions)

1. **Pure visual styling** (CSS) — Playwright screenshot diff is the test.
2. **Spike code in `/spike` branches** — must be deleted before any merge.
3. **External API integrations** — contract tests + recorded responses (vcr-style).
4. **Performance hot paths** — benchmarks ARE the test.
5. **Generated code** (wasm-bindgen output, OpenAPI clients) — test inputs and integrations, not the
   generator's output.
6. **Dependencies' code** — we don't write tests for `loro` or `jumprope`; we write integration
   tests for our usage.

That's it. Six exceptions. Each is documented above.

---

## PR checklist (the wall)

Reviewers verify each of these before approving:

- [ ] Every new function has at least one test
- [ ] No `assert!(true)`, `assert_eq!(x, x)`, or other tautology
- [ ] No test exercises >1 feature without explicit integration framing
- [ ] No test order-dependence; fresh state per test
- [ ] No mock-of-self; mocks only at boundaries
- [ ] Edge cases covered: empty, max, error, boundary numerical
- [ ] Negative cases covered: wrong input is rejected
- [ ] Test names are behavior-oriented (`it_returns_error_when_doc_id_unknown`), not
      implementation-oriented (`test_function_x`)
- [ ] Property tests have non-trivial properties (not "result is non-null")
- [ ] Coverage gate passes (PR comment must show no regression)
- [ ] Mutation kill rate did not drop on the changed layer
- [ ] CHANGELOG updated under `[Unreleased]`
- [ ] CONTRIBUTING / ATTRIBUTION updated if relevant

---

## Commit conventions

[Conventional Commits](https://www.conventionalcommits.org/). Enforced by `commitlint` in
`commit-msg` hook.

```
feat(layout): add line-break for CJK text
fix(format-docx): preserve footnote ordering in round-trip
test(citation): add proptest for empty author field
docs(readme): update getting-started for wasm-pack 0.13
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`,
`revert`, `release`.

---

## Code style

- **Rust:** `cargo fmt` (rustfmt), `cargo clippy -- -D warnings`. Edition 2024. No `unsafe` unless
  genuinely required (and reviewed twice).
- **TypeScript:** dprint format, ESLint (Solid plugin). Strict mode. No `any` without
  `eslint-disable-next-line` + reason.
- **Markdown / JSON / TOML / YAML:** dprint.

Pre-commit hooks (`lefthook`) enforce all of this locally.

---

## Building a plugin

Apalabrar is plugin-extensible. Plugins are **WASM Components** that import a small set of host
functions (read the doc, register a UI panel, fetch over HTTP, render citations) and export a `run`
entry point. The host sandboxes them with capability grants and resource quotas.

The five-minute mental model and a ~30-minute walkthrough that ends with a working word counter are
in [**docs/plugins/getting-started.md**](docs/plugins/getting-started.md). The reference plugin
source is at [`examples/plugin-word-counter/`](examples/plugin-word-counter); a production-shaped
plugin (Zotero / BetterBibTeX importer) is at [`examples/plugin-zotero/`](examples/plugin-zotero) —
it uses every capability the host exposes and serves as the SDK end-to-end test.

---

## Reporting bugs

Open an issue using the `bug` template. Include:

- What you did (the input / steps)
- What you expected
- What actually happened
- Browser + OS
- A failing test, if you can produce one

---

## The Founder Discipline Pact

This pact is signed in spirit by the maintainer and every contributor:

> I will not write a line of implementation code without a failing test that requires it.
>
> I will not modify a test to make it pass; only to correct it.
>
> I will not skip mutation testing on layers where adoption requires trust.
>
> I will not skip the coverage gate to ship a feature faster.
>
> When velocity demands I cut corners, I cut features instead — never tests.
>
> When I am tired and tempted to skip, I remember that ZenDiS asks for these reports.
>
> When I am rushed and tempted to cheat, I remember that academic users will edit a thesis they
> cannot afford to lose.
>
> When I am alone and tempted to ship now, I remember that an editor that quietly corrupts data is
> worse than one that doesn't ship.
>
> Discipline is the moat.

That is all. Print it. Pin it. Welcome aboard.

═════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════

✍️ Signed: Juan Patricio Marchetto, 2026-04-27
