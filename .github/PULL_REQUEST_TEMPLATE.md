# PR — please complete every section

## Summary

(One sentence: what this PR changes and why.)

## Type of change

- [ ] Bug fix
- [ ] New feature (test-first; tests existed BEFORE implementation)
- [ ] Refactor (no behavior change; coverage and mutation kill-rate must NOT regress)
- [ ] Documentation
- [ ] Tooling / CI

## TDD discipline checklist (NON-NEGOTIABLE)

- [ ] Tests were written **before** implementation (commit history shows RED-first)
- [ ] **No test was modified solely to make it pass.** Modified tests are flagged below with
      rationale.
- [ ] No `assert!(true)`, `assert_eq!(x, x)`, or other tautological assertions
- [ ] No tests with mocked system-under-test (mocks only at external boundaries)
- [ ] Edge cases covered: empty input, max input, error path, boundary numerical values
- [ ] If touching CRDT / parser / layout: at least one property-based test added or extended
- [ ] If touching format I/O: golden round-trip snapshot updated and reviewed

## Modified tests (if any)

| File | Reason for modification |
| ---- | ----------------------- |
|      |                         |

## Coverage delta

- Line coverage: % (Δ )
- Branch coverage: % (Δ )

## Mutation kill-rate (changed files only)

- Kill rate: % (Δ )
- Surviving mutations: (none / list with file:line + reason kept)

## Bundle size delta (if app touched)

- Before: KB compressed
- After: KB compressed

## Manual verification

- [ ] `cargo test --workspace` — all green
- [ ] `pnpm test` — all green
- [ ] `pnpm typecheck` — clean
- [ ] `pnpm e2e` — passes locally
- [ ] Tested on a 4 GB Chromebook (or equivalent throttle profile) if perf-relevant
- [ ] Spanish/Portuguese keyboard composition tested if input-relevant

## Linked issues / docs

Closes # (issue id) Blueprint section: (file + section if architectural)

## Reviewer focus

(What part of the diff most needs careful eyes?)
