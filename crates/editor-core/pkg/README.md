# Apalabrar

> A browser-native, local-first academic document editor — Rust core, WASM runtime, Solid.js shell.
> Word-class fidelity without the desktop tax.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![WASM](https://img.shields.io/badge/runtime-WASM-654FF0.svg)](https://webassembly.org)
[![Solid.js](https://img.shields.io/badge/UI-Solid.js-2c4f7c.svg)](https://www.solidjs.com)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg)](https://conventionalcommits.org)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

> **Status:** pre-alpha. Day-1 scaffolding. The first failing test ships with commit number two.

---

## Quality dashboard

Auto-updated nightly (badges go live as CI lands):

| Metric                                      | Status        |
| ------------------------------------------- | ------------- |
| Tests                                       | _coming soon_ |
| Line coverage                               | _coming soon_ |
| **Mutation kill rate** (the metric)         | _coming soon_ |
| Bundle size                                 | _coming soon_ |
| E2E pass rate (Chromium / Firefox / WebKit) | _coming soon_ |
| DOCX round-trip fidelity (100-doc corpus)   | _coming soon_ |
| p95 keystroke latency (Chromebook 4 GB)     | _coming soon_ |

These badges are public and PR-blocking. They are also a **procurement signal**: EU institutional
buyers (ZenDiS, SURF, DINUM) request coverage and mutation reports as part of evaluation.

---

## What it is

Apalabrar is what an academic document editor should look like in 2026:

- **Local-first.** Your documents live in OPFS / IndexedDB / your filesystem. No server is required
  to read, edit, or save a thesis.
- **Word-class fidelity.** Round-trip OOXML (`.docx`) without losing footnotes, comments, track
  changes, equations, or bibliographic references.
- **Citation-native.** Built-in CSL engine, Zotero plugin, BetterBibTeX bridge — so footnotes _just
  work_.
- **CRDT-first.** Powered by [Loro](https://loro.dev) — real-time collaboration without a sync
  server when you don't need one, with a sync server when you do.
- **AI-optional.** Local-first opt-in (Phi-3 WebGPU, Whisper.cpp) by default; cloud BYO-key when you
  want it. You own the privacy posture.
- **Plugin-extensible.** WASM Component Model sandbox. Capability-scoped. You install a plugin; the
  plugin gets exactly the privileges it asked for.

## What it is not

- Not Notion. Native footnotes, page numbers, cross-references, paginated layout. Theses, not
  blocks.
- Not a Word fork. Greenfield Rust core. Inspired by — and credits — many.
- Not server-required. Self-host with `docker-compose`, or just open the static SPA and write.

---

## Architecture in 30 seconds

```
┌─────────────────────────────────────────────────────────────────┐
│  Solid.js shell  ←→  TypeScript bridge  ←→  Rust core (WASM)    │
│                                                                  │
│  • UI components               • Loro CRDT       • DOCX I/O      │
│  • Toolbar, sidebar            • jumprope        • Markdown      │
│  • Modals, find/replace        • cosmic-text     • CSL citation  │
│  • Tailwind 4, kobalte         • rustybuzz       • Plugin host   │
└─────────────────────────────────────────────────────────────────┘
                              ↑                ↑
                        OPFS storage      Optional sync
                                          server (Loro)
```

Full design: see `/home/marche/.pain-scout/blueprint-part3-synthesis.md` (synthesis blueprint,
locked 2026-04-26).

## Repository layout

```
apalabrar/
├── crates/                Rust workspace
│   ├── editor-core/       Umbrella public API (compiles to WASM)
│   ├── doc-model/         Loro CRDT wrapper
│   ├── layout/            Paged layout, line-break, pagination
│   ├── format-docx/       OOXML read/write — the moat
│   ├── format-md/         Markdown (pulldown-cmark + comrak)
│   ├── format-html/       HTML (html5ever)
│   ├── format-odt/        OpenDocument (stub, v1)
│   ├── format-rtf/        RTF (stub, v1)
│   ├── citation/          citeproc-rs wrapper
│   ├── plugin-host/       WASM Component Model sandbox
│   └── ai-bridge/         Local + BYO-key AI providers
│
├── packages/              JS workspace
│   ├── editor-bridge/     TypeScript bindings to WASM
│   ├── ui/                Solid components (Tailwind 4, kobalte)
│   ├── app/               The actual SPA (Vite + Solid)
│   └── server/            Sync server (deferred to v1)
│
├── tests-corpus/          OOXML golden corpus (100 → 500 docs)
├── tests-e2e/             Playwright multi-browser suite
├── .github/workflows/     CI / nightly / release
```

---

## Getting started

Prerequisites:

- Rust stable 1.85+ (auto-pinned via `rust-toolchain.toml`)
- Node.js 20.11+
- pnpm 9+
- `wasm-pack` (`cargo install wasm-pack`)

```bash
# Clone
git clone https://github.com/apalabrar/apalabrar.git
cd apalabrar

# Install JS dependencies
pnpm install

# Build the Rust workspace (host target)
cargo build --workspace

# Run all tests
cargo test --workspace
pnpm test

# Type-check everything
pnpm typecheck

# Start the dev server (Vite + Solid)
pnpm dev
# → open http://localhost:5173
```

You should see **"Apalabrar — coming soon"**. That's the day-1 finish line.

### Development tasks

| Command          | Purpose                                        |
| ---------------- | ---------------------------------------------- |
| `pnpm dev`       | Start Vite dev server for `packages/app`       |
| `pnpm build`     | Build all packages (Turbo)                     |
| `pnpm test`      | Run all unit + component tests (Rust + Vitest) |
| `pnpm e2e`       | Run Playwright E2E suite                       |
| `pnpm typecheck` | TypeScript no-emit check across packages       |
| `pnpm lint`      | ESLint + Clippy                                |
| `pnpm format`    | dprint format everything                       |
| `pnpm coverage`  | Coverage reports (cargo-llvm-cov + vitest)     |
| `pnpm mutants`   | Mutation testing (slow — nightly in CI)        |
| `cargo bench`    | Criterion benchmarks                           |

---

## Discipline

This project ships with **furious TDD discipline** — no implementation code is written without a
failing test that requires it. Read `CONTRIBUTING.md` before your first PR; it documents the rules
and the Founder Discipline Pact.

Mutation testing (`cargo-mutants`) is the floor metric. Line coverage is the guideline. **Surviving
mutations block PRs.**

---

## Inspirations & dependencies

This is greenfield code, but it stands on excellent shoulders. See
[`ATTRIBUTION.md`](ATTRIBUTION.md) for the full list (eigenpal, SuperDoc, LaSuite, Typst, AppFlowy,
Loro, jumprope, cosmic-text, citeproc-rs, etc.).

If you fork Apalabrar, please keep `ATTRIBUTION.md` intact — the upstream projects deserve the
credit.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) — TDD rhythm, code review checklist, and the Founder
Discipline Pact. Then look at the issue tracker for `good-first-issue` labels.

## Security

Found a vulnerability? See [`SECURITY.md`](SECURITY.md). Please do not open a public issue.

## License

MIT. See [`LICENSE`](LICENSE). Apalabrar will never relicense to AGPL — fork freedom is
non-negotiable.

---

_Apalabrar — to give your word._
