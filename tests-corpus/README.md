# tests-corpus — OOXML round-trip golden corpus

This directory holds `.docx` files used as golden inputs for round-trip fidelity tests.

## Categories (target taxonomy)

- `academic/` — Thesis chapters, journal articles, citation-heavy
- `legal/` — Bluebook citations, complex tables of authorities
- `multilingual/` — Spanish, Portuguese, German, French, Arabic, Chinese (RTL/CJK testing)
- `equations/` — Math-heavy with MathML / OMML
- `tables/` — Complex tables, merged cells, nested
- `tracked/` — Track-changes + comments
- `legacy/` — `.doc` (Word 97-2003) for compatibility tests

## Goals

- v0 milestone: 100 docs total, ≥95% byte-equivalent round-trip
- v1 milestone: 500 docs total, ≥99% byte-equivalent round-trip on simple categories

## How to add a doc

1. Add `.docx` to the appropriate category subdirectory.
2. Run `cargo test --release --test corpus_round_trip -- --ignored` to generate a snapshot.
3. Review the snapshot diff and `cargo insta accept` if the baseline is correct.
4. Commit both the `.docx` and the snapshot.

## What NOT to commit

- Files that contain real PII (use anonymized fixtures only).
- Files larger than 5 MB (compress or split).
