# Attribution

Apalabrar is greenfield code, but it stands on the shoulders of many excellent projects. This
document credits those projects — their ideas, their code, or both.

If you fork Apalabrar, please keep this file intact. The upstream maintainers deserve the credit;
you can add your own attributions on top.

---

## Inspirations (ideas, architecture, prior art)

| Project                                              | What we learned                                            | License          |
| ---------------------------------------------------- | ---------------------------------------------------------- | ---------------- |
| [eigenpal](https://github.com/eigenpal/eigenpal)     | Browser-native editor architecture; OPFS-first storage     | MIT              |
| [SuperDoc](https://github.com/superdoc-dev/superdoc) | DOCX round-trip approach in JS                             | AGPL (we differ) |
| [LaSuite (DINUM)](https://github.com/numerique-gouv) | Government-grade office suite; institutional sale playbook | MIT / public     |
| [Typst](https://typst.app)                           | Layout engine, paginated rendering, academic typesetting   | Apache-2.0       |
| [AppFlowy](https://github.com/AppFlowy-IO/AppFlowy)  | Plugin sandbox patterns; Rust-core architecture            | AGPL (we differ) |
| [OnlyOffice](https://www.onlyoffice.com)             | OOXML compatibility playbook (and the AGPL trap to avoid)  | AGPL (we avoid)  |
| [Zotero](https://www.zotero.org)                     | Citation workflow; the original "academic-grade" reference | AGPL / MPL       |
| [Quarto](https://quarto.org)                         | Author-first publishing UX; academic doc semantics         | MIT              |

---

## Direct dependencies (Rust)

| Crate                                                           | Purpose                         | License          |
| --------------------------------------------------------------- | ------------------------------- | ---------------- |
| [`loro`](https://github.com/loro-dev/loro)                      | CRDT (rich-text, Peritext)      | MIT              |
| [`jumprope`](https://github.com/josephg/jumprope-rs)            | Rope buffer                     | ISC              |
| [`rustybuzz`](https://github.com/RazrFalcon/rustybuzz)          | HarfBuzz port (text shaping)    | MIT              |
| [`cosmic-text`](https://github.com/pop-os/cosmic-text)          | Text layout                     | MIT              |
| [`docx-rs`](https://github.com/bokuweb/docx-rs)                 | OOXML read/write                | MIT              |
| [`pulldown-cmark`](https://github.com/raphlinus/pulldown-cmark) | Markdown parsing                | MIT              |
| [`comrak`](https://github.com/kivikakk/comrak)                  | Markdown rendering (CommonMark) | BSD-2-Clause     |
| [`html5ever`](https://github.com/servo/html5ever)               | HTML parsing                    | MIT / Apache-2.0 |
| [`quick-xml`](https://github.com/tafia/quick-xml)               | Fast XML parsing                | MIT              |
| [`citeproc-rs`](https://github.com/zotero/citeproc-rs)          | CSL citation engine             | MPL-2.0          |
| [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen)      | Rust↔JS bindings                | MIT / Apache-2.0 |
| [`thiserror`](https://github.com/dtolnay/thiserror)             | Error derivation                | MIT / Apache-2.0 |
| [`serde`](https://github.com/serde-rs/serde)                    | Serialization                   | MIT / Apache-2.0 |
| [`tokio`](https://github.com/tokio-rs/tokio)                    | Async runtime (server only)     | MIT              |

### Test infrastructure

| Crate                                                          | Purpose                 | License          |
| -------------------------------------------------------------- | ----------------------- | ---------------- |
| [`proptest`](https://github.com/proptest-rs/proptest)          | Property-based testing  | MIT / Apache-2.0 |
| [`insta`](https://github.com/mitsuhiko/insta)                  | Snapshot testing        | Apache-2.0       |
| [`criterion`](https://github.com/bheisler/criterion.rs)        | Benchmarking            | MIT / Apache-2.0 |
| [`cargo-mutants`](https://github.com/sourcefrog/cargo-mutants) | Mutation testing        | MIT              |
| [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov)  | Coverage                | MIT / Apache-2.0 |
| [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)        | Coverage-guided fuzzing | MIT / Apache-2.0 |

---

## Direct dependencies (JS)

| Package                                                                    | Purpose                      | License    |
| -------------------------------------------------------------------------- | ---------------------------- | ---------- |
| [Solid.js](https://github.com/solidjs/solid)                               | Reactive UI framework        | MIT        |
| [kobalte](https://github.com/kobaltedev/kobalte)                           | Headless WAI-ARIA components | MIT        |
| [Tailwind CSS 4](https://github.com/tailwindlabs/tailwindcss)              | Utility-first CSS            | MIT        |
| [Vite](https://github.com/vitejs/vite)                                     | Bundler / dev server         | MIT        |
| [vite-plugin-solid](https://github.com/solidjs/vite-plugin-solid)          | Solid integration            | MIT        |
| [TypeScript](https://github.com/microsoft/TypeScript)                      | Type system                  | Apache-2.0 |
| [Vitest](https://github.com/vitest-dev/vitest)                             | Unit testing                 | MIT        |
| [@testing-library/solid](https://github.com/solidjs/solid-testing-library) | Component testing            | MIT        |
| [Playwright](https://github.com/microsoft/playwright)                      | E2E browser testing          | Apache-2.0 |
| [axe-core](https://github.com/dequelabs/axe-core)                          | Accessibility testing        | MPL-2.0    |
| [Turborepo](https://github.com/vercel/turbo)                               | Monorepo task runner         | MPL-2.0    |
| [pnpm](https://github.com/pnpm/pnpm)                                       | Package manager              | MIT        |
| [dprint](https://github.com/dprint/dprint)                                 | Code formatter               | MIT        |
| [lefthook](https://github.com/evilmartians/lefthook)                       | Git hooks                    | MIT        |
| [commitlint](https://github.com/conventional-changelog/commitlint)         | Commit message linting       | MIT        |

---

## Build & developer experience

| Tool                                                  | Purpose              | License          |
| ----------------------------------------------------- | -------------------- | ---------------- |
| [`wasm-pack`](https://github.com/rustwasm/wasm-pack)  | WASM build toolchain | MIT / Apache-2.0 |
| [`wasm-opt`](https://github.com/WebAssembly/binaryen) | WASM optimizer       | Apache-2.0       |

---

## Notes

- We deliberately avoid AGPL dependencies in the **product** path because AGPL is fatal to B2B EU
  institutional sales (per the OnlyOffice / Nextcloud Euro-Office case study). AGPL inspirations are
  listed for credit but their code is not vendored.
- All MPL-2.0 dependencies (`citeproc-rs`, `axe-core`) are file-level copyleft and compatible with
  our MIT distribution.
- `comrak` is BSD-2-Clause and compatible with MIT.

If we missed an attribution, please open a PR. Credit matters.
