# Changelog

All notable changes to Apalabrar are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- Phase 4 prompt 4.1 — `layout` crate refactored to the canonical
  Phase 4.1 surface. New free-fn `pub fn layout(doc: &Doc, viewport:
  &Viewport) -> Result<RenderPlan, Error>` consumes the doc-model
  CRDT (`apalabrar-doc-model::Doc`) and emits the blueprint Section G
  `RenderPlan` shape (`pages` + `dirty_rects`; `glyph_runs` /
  `selections` / `carets` deferred to 4.2 on `#[non_exhaustive]`).
  The legacy Gate 5 surface (`Engine` / `Document` /
  `LaidOutDocument`) is removed; cosmic-text + page-pack machinery
  stay private. Internal optimisation: thread-local `ShapingCache`
  reuses the `FontSystem` and per-block shaped lines across calls
  (the public surface is stateless from the caller's perspective)
  so re-laying out the same doc hits 100 % cache and only re-packs
  pages. doc-model gains `Doc::blocks() -> Vec<Block>` — a bulk
  projection that does the body-clone and `\n`-split exactly once
  instead of per-block. 73 tests cover unit (41 api + 8 traits + 2
  lib), integration via the doc-model CRDT (10), insta snapshots
  (5), and proptest properties (7 × 256 random docs each =
  page-count monotonicity per blueprint Section E R3, conservation,
  permutation, determinism, dirty-rect cardinality, line-width
  bounds, kind round-trip). Acceptance: 200-page doc layout p95 =
  **4.36 ms** (11.5× under the 50 ms gate criterion). Coverage:
  97.24 % region / 100 % function / 95.38 % line. Mutation kill:
  50 / 55 viable = **90.91 %** (above the 90 % non-moat floor); the
  5 survivors are 3 cache-overflow-timing equivalents on line 177
  and 2 float-EPSILON perturbations on `pack_pages` line 417 (same
  documented equivalents as Gate 5 stage 1).
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
- Phase 3 prompt 3.5 — `format-html` HTML read/write under TDD discipline.
  Adds `pub fn read_html(s) -> Result<DocModel, Error>` + `pub fn write_html(doc)
  -> Result<String, Error>`. Same `BlockKind` taxonomy as format-md
  (Paragraph, Heading 1-6, List, BlockQuote, CodeBlock, Table,
  ThematicBreak). Unlike format-md/format-docx the original source is
  NOT held verbatim — the entire point of `write_html` is the
  controlled subset emission, so pasted noise (scripts, styles, classes,
  inline event handlers, data-* attributes) is dropped by construction.
  Parser uses html5ever 0.29 + markup5ever_rcdom 0.5.0-unofficial:
  `parse_document` builds an RcDom; `find_body` descends to `<body>`;
  per body child, `classify_top_level` maps element local name to a
  `BlockKind` and `collect_text` recursively concatenates text nodes
  while skipping `<script>` / `<style>` subtrees (mirroring browser
  `Element.textContent`). Writer emits a minimal allow-list of tags
  (`h1`-`h6`, `p`, `ul`/`li`, `blockquote`, `pre`, `table`/`tr`/`td`,
  `hr`) with HTML-escaped text content. 47 new tests covering element
  classification, sanitization (script/style/class/style/data-*
  stripping), HTML-special-char escape on save, multibyte (LATAM, CJK),
  edge cases (empty, whitespace-only, OOB indices), 16-fixture
  round-trip on paragraph_count + block_kind + paragraph_text, and 3
  proptest properties (256 cases each) on never-panic, index
  resolution, and editable-surface preservation. Also extends the
  Phase 3.4 corpus-roundtrip CI gate to include the format-html spec
  test.
- Phase 3 prompt 3.4 — `corpus-roundtrip` CI gate. New PR-blocking job
  in `.github/workflows/ci.yml` runs the OOXML round-trip + insta
  snapshot + `read_preserve` / `write_preserve` tests for `format-docx`
  AND the round-trip + spec tests for `format-md` as a single focused
  signal. `INSTA_OUTPUT=diff` + `INSTA_UPDATE=no` ensure any drifted
  snapshot fails the test with a unified diff in the log. On PR
  failure, the job uploads the per-crate test logs as artifacts and
  posts a comment carrying the last 8 KB of each log + a recipe to
  reproduce locally and (only after manual review) `cargo insta
  accept` the change. 161 tests run under this gate (112 from
  format-docx + 49 from format-md). Verifying that a deliberately
  corrupted fixture trips the gate is left as a manual recipe in the
  Phase 3.4 memory note — making a real broken-on-purpose PR is the
  only honest end-to-end test.
- Phase 3 prompt 3.3 — `format-md` Markdown read/write under TDD discipline.
  Adds `pub fn read_md(s) -> Result<DocModel, Error>` + `pub fn write_md(doc)
  -> Result<String, Error>`. The structural model classifies top-level
  blocks (Paragraph / Heading(1-6) / List / BlockQuote / CodeBlock / Table
  / ThematicBreak / FootnoteDefinition) and projects each to inline text;
  the original source is held verbatim so `write_md` round-trips
  byte-equivalent on unmodified models — same lossless-by-default
  pattern Phase 3.1/3.2 established for `format-docx`.
  Parser uses `pulldown-cmark` 0.12 with all GFM extensions enabled
  (tables, footnotes, strikethrough, task lists). 50 new tests:
  44 spec + GFM unit tests covering ATX/Setext headings (1-6),
  paragraphs with inline emphasis/code/links/images, ordered + unordered
  + nested lists, single + multi-line block quotes, fenced + indented
  code blocks, all three thematic-break syntaxes, multi-block
  documents, empty/whitespace edge cases, LATAM diacritics + CJK,
  GFM pipe tables, GFM footnotes (ref + definition), GFM task lists,
  and GFM strikethrough; 5 round-trip tests asserting byte-equivalent
  emit + paragraph_count + block_kind + paragraph_text preservation
  across 23 fixtures; 2 proptest properties (256 cases) on never-panic
  + index-resolution; 1 round-trip property (128 cases) on editable-
  surface preservation.
- Phase 3 prompt 3.2 — `format-docx` write path under TDD discipline. Adds
  `pub fn write_preserve(doc, shadow) -> Result<Vec<u8>, Error>` mirroring
  `read_preserve`: the `ShadowXml` snapshot is the source of truth for every
  zip part the structural model didn't touch, while `DocModel`-side
  paragraph mutations are spliced into the shadow's `word/document.xml`.
  For an unmutated `DocModel`, `write_preserve` produces bytes
  byte-equivalent (after XML normalization) to those that produced
  `(doc, shadow)`. Refactor: `write` and `write_preserve` now share an
  `effective_document_xml` decision helper plus a `build_zip` emitter so
  the moat-critical zip writing has a single test/mutation surface.
  9 new tests in `tests/write_preserve.rs`: model-equivalent round-trip
  via `write` across all 26 fixtures (with and without mutation),
  byte-equivalent lossless round-trip via `write_preserve` across all
  26, "non-document parts come from shadow" probe, dirty-document-xml
  re-read probe, equivalence between `write` and `write_preserve` on
  unmutated models, plus two proptest properties (valid OOXML
  losslessly round-trips, single-paragraph mutation propagates
  correctly). Coverage on the new layer is 100% line; the only
  uncovered lines in the crate (56, 59) are pre-existing
  `parse_text` gaps from Phase 1 unrelated to this prompt.
- Phase 3 prompt 3.1 — `format-docx` read path under TDD discipline. Adds
  `pub fn read_preserve(bytes) -> Result<(DocModel, ShadowXml), Error>`
  alongside the existing `read`. `ShadowXml` is a verbatim snapshot of every
  zip part (recognized or not) captured at read time, so callers always have
  a lossless escape hatch even after arbitrary `DocModel` mutations.
  `read` is now a thin `read_preserve(bytes).map(|(m, _)| m)` wrapper —
  single source of truth, identical semantics. 42 new tests in
  `tests/read_preserve.rs`: 26 ShadowXml insta snapshots (one per corpus
  fixture, all manually reviewed before `cargo insta accept`), a per-fixture
  lossless modify-then-write assertion across all 26 docs, per-fixture
  shadow-byte and document-xml equivalence with the underlying zip,
  read↔read_preserve agreement on paragraph_count and paragraph_text,
  three error-path tests (empty bytes, garbage, zip without
  `word/document.xml`), a "shadow survives DocModel mutation" test, and
  five proptest properties (random-bytes never-panics + DocModel
  index-resolves invariant + part-count agrees with zip + valid OOXML
  via `serialize_text` round-trips + read↔read_preserve always agree).
  Total format-docx test count: 36 round-trip + 25 snapshot + 9 api +
  2 properties + 42 read_preserve = 114 tests, all green.
- Phase 2 prompt 2.6 — test corpus expansion to 25 fixtures across
  six categories. The 5-fixture Validation Gate 4 starter set is
  joined by 20 LibreOffice-generated docs covering: 3 academic
  variations (numbered list, block quote, mixed emphasis), 3
  multilingual additions (pt-BR, French+German+Italian, mixed
  Greek/Latin/Slavic scripts), 3 table variations (rowspan+colspan
  merge, multi-row demographic header, numeric-only stats), 3
  footnote variations (multiple footnotes per paragraph, formatted
  footnote body, cross-paragraph anchors), 4 equation fixtures
  (Unicode math inline, Greek symbols + physics constants, display
  math with sub/superscripts, equations interleaved with prose),
  and 4 tracked-change fixtures (insertion, deletion of a whole
  paragraph, two comments, mixed insertion + comment). All 20 new
  fixtures round-trip byte-equivalent through the format-docx
  shadow-store after whitespace + attribute-order normalization,
  bringing the round-trip test count from 16 to 36 (plus the 1
  synthetic edge case). 20 new insta snapshots capture the
  paragraph-text projection of each fixture; manually inspected
  before `cargo insta accept`. The expanded corpus is the
  foundation for the v0 milestone target of 100 docs / ≥95%
  byte-equivalent round-trip.
- Phase 2 prompt 2.5 — frictionless UX page-load flow: ships the
  Section D timeline as the new `/` route. `index.html` inlines a
  critical CSS skeleton so the blank-doc surface paints at T+0
  (before any JS runs); the Solid `Landing` page swaps in once
  hydrated and runs two parallel bootstraps — `bootstrapOpfs` (OPFS
  scan, returns recents sorted newest-first or `{storage: null,
  recents: []}` if OPFS is unavailable) and `bootstrapCore` (WASM
  init, deferred so the editable surface is interactive before the
  editor finishes loading). A `KeystrokeBuffer` (push / drain /
  size / closed / close) holds keystrokes typed during the
  T+100→T+200 window and replays them on core-ready, so no input
  is lost. New components: `ContinueToast` (role=status,
  aria-live=polite, default 5 s auto-dismiss with cancellation on
  manual continue/dismiss) and `RecentDocsMenu` (aria-haspopup
  disclosure, formatted size in B/KB/MB; renders nothing when the
  recents list is empty so the toolbar stays clean for first-time
  visitors). 37 vitest tests (10 KeystrokeBuffer + 5 bootstrapOpfs
  + 8 ContinueToast + 6 RecentDocsMenu + 8 Landing) and 12
  Playwright E2E tests (5 page-load timing/skeleton/focusability,
  1 toast clean-OPFS, 2 journey #1 first-time visitor, 1 journey
  #2 recents-absent, 2 axe-core a11y on landing + composer) for a
  combined Phase 2.5 contribution of 49 tests. Vitest coverage on
  the new files: **100 % statement / 100 % branch / 100 % function
  / 100 % line** on `keystrokeBuffer.ts`, `opfsBootstrap.ts`,
  `RecentDocsMenu.tsx`; **100 % line / 89 % branch** on
  `ContinueToast.tsx`; **100 % line / 100 % branch / 83 %
  function** on `Landing.tsx` (the one uncovered function is the
  `RecentDocsMenu` `onSelect` callback wired to a recent-doc click
  — exercised by E2E only since the menu is closed when recents
  are empty). axe-core reports zero serious/critical violations on
  every state. Critical user journeys #1 and #2 (first-time
  visitor, returning user) are fully covered; #3 (drag-drop .docx)
  is partial via the existing `format-docx` parser; #4-#10 (auth,
  collab, plugins, AI, DOCX export round-trip, self-host) are
  documented as deferred to v1+ phases.
- Phase 2 prompt 2.4 — EditOp dispatcher producing `RenderDelta`:
  `apalabrar-editor-core` gains a `dispatch` module with the
  signature `pub fn dispatch(doc: &mut Doc, op: EditOp) ->
  Result<RenderDelta, Error>`. `RenderDelta` carries the seam
  between "apply this op" and "tell the renderer what changed":
  `dirty_blocks: BlockRange { start, end }` (half-open block-index
  range to reflow), `structural: bool` (true when block_count
  changed — pagination/scroll invalidation hint), `caret_hint:
  Option<Position>` (codepoint position for the caret post-op,
  `None` for `FormatRange` / `InsertComment` / `Suggest` /
  `AcceptSuggestion` which don't move the caret), and `minted_id:
  Option<MintedId>` (annotation id minted by `InsertComment` /
  `Suggest` / `InsertCitation` / `InsertFootnote` so the UI can
  attach without re-querying). `RenderDelta::NOOP` is the sentinel
  for clip-defensive no-ops (empty text insert, zero-width range,
  inverted `from > to`, non-adjacent merge). The dispatcher
  mirrors doc-model clip semantics (`from > to` is no-op, not a
  swap). Doc-model errors propagate as `Error::EditOpFailed { kind,
  reason }` carrying the variant name + the doc-model error's
  `Display` so JS callers can branch without exhaustive matches.
  42 host-side contract tests (`crates/editor-core/tests/dispatch.rs`):
  3 sentinel constants + 7 InsertText + 5 DeleteRange + 3 FormatRange
  + 3 InsertBlock + 3 SplitBlock + 3 MergeBlocks + 2 InsertComment
  + 1 Suggest + 4 AcceptSuggestion + 2 InsertCitation + 2 InsertFootnote
  + 3 proptest properties (64 cases each: dispatch is total, doc
  remains consistent, dirty range is well-formed). cargo-mutants
  on `dispatch.rs`: **61 caught / 0 missed / 1 unviable = 100 %
  kill** on viable mutations. cargo-llvm-cov: **100 % function
  / 99.29 % line / 97.75 % region** (the 2 missed lines are
  defensive `?`-error-propagation arms on `apply()` calls for
  `InsertComment` and `Suggest`, where the doc-model handlers
  never error in practice — kept for forward compatibility).
- Phase 2 prompt 2.3 — JS↔Rust bridge (`ApalabrarCore`): the
  `apalabrar-editor-core` crate gains a `bridge` module that
  exposes the doc-model `EditOp` surface (the full 11-variant
  blueprint Section G enum) through a JSON dispatcher suitable for
  the wasm-bindgen boundary. The `apalabrar-doc-model` types
  (`Mark`, `BlockKind`, `Block`, `BlockTree`, `EditOp`, `Comment`,
  `Suggestion`, `SuggestionState`, `Citation`, `Footnote`) gain
  `serde::{Serialize, Deserialize}` derives. `EditOp` uses
  `#[serde(tag = "kind")]` and `BlockKind` uses `#[serde(tag =
  "type")]` so the JSON shape matches the blueprint's TypeScript
  declaration verbatim. Public Rust API (in `editor_core::bridge`):
  `create_doc()`, `apply_edit_op(doc_id, op)`, `apply_edit_op_json
  (doc_id, op_json)`, `snapshot(doc_id)`, `restore_from_snapshot
  (bytes)`, `block_count(doc_id)`, `block_at_json(doc_id, idx)`.
  Two new `#[non_exhaustive]` Error variants land:
  `JsonParseFailed { reason }` (malformed input JSON),
  `EditOpFailed { kind, reason }` (doc-model dispatcher failure
  like `AcceptSuggestion` with an unknown id). The wasm-bindgen
  exports (`createDoc`, `applyEditOp`, `bridgeSnapshot`,
  `restoreFromSnapshot`, `blockCount`, `blockAt`, `bridgeDocText`,
  `bridgeCloseDoc`) sit alongside the pre-existing OOXML round-trip
  exports — both surfaces coexist and consume the same registry.
  TypeScript facade `ApalabrarCore` ships in
  `@apalabrar/editor-bridge` (new `core.ts` module), wrapping the
  wasm exports with a typed object surface (`EditOp` discriminated
  union, `Block` / `BlockKind` / `Mark` types, `CoreDocId` branded
  bigint). The wasm module is constructor-injected so callers and
  tests can supply a mock without touching the actual binary. 33
  GREEN host-side Rust contract tests in `tests/bridge.rs` (one
  happy path per EditOp variant + 4 lifecycle + 6 JSON dispatch +
  3 query + 4 error + 2 proptest properties) plus 21 GREEN vitest
  contract tests in `core.test.ts` (5 lifecycle + 11 EditOp
  serialisation + 4 query + 1 fast-check round-trip property
  covering all 11 variants). cargo-mutants (with `wasm_api::`
  module excluded — pass-through wrappers smoke-tested in CI)
  reports **14/14 = 100 %** kill on the host-testable bridge
  surface; cargo-llvm-cov reports **100 % line / 100 % function /
  100 % branch on `bridge.rs`** (matches the prompt's "100 % line
  + branch coverage required" gate). Vitest v8 coverage on
  `core.ts` is **100 % statement / 100 % branch / 100 % function
  / 100 % line**. Build environment limitation: the wasm pkg
  cannot be rebuilt locally because `clang` is unavailable to
  cc-rs in CI's sandbox; the wasm exports are syntactically
  correct and gated on `cfg(target_arch = "wasm32")`, and the
  pkg rebuild + browser-runtime wasm-bindgen-test smoke run land
  in a follow-up commit once CI provides clang. The TS facade
  declares the wasm shape via `ApalabrarCoreWasm` interface so it
  typechecks and tests run today; the runtime swap to the real
  pkg is a one-line constructor change.
- Phase 2 prompt 2.2 — doc-model citations + footnotes (Phase D —
  InsertCitation / InsertFootnote): the doc-model crate completes
  the EditOp surface. **All 11 blueprint variants now have real
  handlers** — no `Err(Error::NotYetImplemented(..))` path remains
  in `apply_edit_op`. The `NotYetImplemented` variant stays in the
  public Error surface for forward-compat (any future EditOp
  addition has a place to land while its handler is being written),
  but no current variant produces it. Citations and footnotes
  represent themselves as private-use-area marker codepoints
  embedded in the body text — `\u{E000}` for citations, `\u{E001}`
  for footnotes. Distinct codepoints (vs one shared marker) means
  the layout engine and OOXML serialiser can dispatch on the
  codepoint alone without a parallel-map lookup. Records live in
  `citations` (sub-LoroMap with `at` + `key`) and `footnotes`
  (sub-LoroMap with `at` + `blocks`) root maps; ID generation
  reuses the Phase C peer-id-prefixed-counter machinery with two
  new meta keys. Footnote bodies serialise as a `LoroValue::List`
  of `LoroValue::Map<{kind, text}>` — flat, not recursive
  (Phase A's `BlockTree` shape is preserved); the recursive case
  ships when layout / OOXML round-trip needs it. Public API gains
  `Citation` and `Footnote` types plus 6 accessors
  (`citation_ids`, `citation`, `last_citation_id`, `footnote_ids`,
  `footnote`, `last_footnote_id`). Anchor caveat unchanged from
  Phase C: positions are RAW codepoints. Phase A's two
  `*_returns_not_yet_implemented` tests for these variants were
  rewritten as happy-path; the `prop_deferred_variants_preserve_text`
  property and `deferred_variants_do_not_mutate_the_doc` sweep
  have RETIRED — they have no still-deferred variants to cover.
  109 GREEN edit-op tests + 3 lib unit tests + 20 api carry-over
  + 5 CRDT properties = 137 in the doc-model crate (Phase C was
  115). cargo-mutants kills 150/151 = **99.34 %** on the moat layer
  with ZERO new survivors from Phase D code; the lone outstanding
  survivor is the equivalent mutant in `handle_split_block`
  carried over from Phase B and already documented inline.
  cargo-llvm-cov reports 97.03 % region / 95 % function / 97.22 %
  line; the 13 uncovered lines are all forward-compat defensive
  arms — `has_mark` per-Loro-contract, malformed-entry fall-throughs
  in `read_*_field` / `read_block_tree_field` helpers, the per-block
  default in `read_block_tree_field`, comment / citation / footnote
  accessor `_ => return None` shape-mismatch arms, and the
  `let-else` no-newline-found fallback in `handle_merge_blocks`.
  doc-model edit-op surface ships fully complete with this commit.
- Phase 2 prompt 2.2 — doc-model comments + suggestions (Phase C —
  InsertComment / Suggest / AcceptSuggestion): the
  `apalabrar-doc-model` crate gains comment-thread and
  track-changes-style suggestion handling. Three more variants
  promote out of `Err(NotYetImplemented(..))` — only the Phase D
  citation + footnote variants remain deferred. Comments live in a
  root `LoroMap` named `"comments"` keyed by `thread_id`; each value
  is a sub-`LoroMap` with `from`/`to` (I64 codepoint positions) and
  `body` fields. Suggestions live in a parallel `"suggestions"`
  `LoroMap` keyed by suggestion id with `from`/`to`/`replacement`/
  `state` fields; state cycles `pending → accepted` (the
  `rejected` string and `SuggestionState::Rejected` variant are
  reserved for a future `RejectSuggestion` op the blueprint will
  add). ID generation uses a persistent `"meta"` `LoroMap` of
  monotonically-increasing i64 counters keyed by op kind; ids are
  formatted `{prefix}-{peer_id_hex}-{counter}` so they survive
  snapshot round-trips and never collide across peers. The
  generated id is exposed on the `Doc` instance via transient
  `last_comment_thread_id()` / `last_suggestion_id()` accessors so
  callers don't need a different return shape on `apply_edit_op` —
  they read it immediately after the op. `Doc::comment_thread_ids
  ()` / `Doc::suggestion_ids()` (sorted) and `Doc::pending_suggestion_ids
  ()` round out the read API alongside per-id `Doc::comment(id)` /
  `Doc::suggestion(id)`. `EditOp::AcceptSuggestion` is idempotent on
  already-accepted records and returns `Err(Error::SuggestionNotFound
  (id))` — a new `#[non_exhaustive]` Error variant — for unknown ids.
  Anchor caveat: positions are RAW codepoints, so anchors go stale
  if surrounding text is edited. Phase C+ will migrate to Loro-
  cursor-based stable anchors when the public Loro 1.12 surface is
  extended (`Cursor`/`Side` are not currently `pub use`-exported).
  Phase A's three `*_returns_not_yet_implemented` tests for these
  variants were rewritten as happy-path tests; the deferred-variants
  property + sweep now only cover the 2 still-deferred Phase D ops.
  87 GREEN edit-op tests (+ 3 lib unit tests for
  `state_to_str`/`str_to_state` round-trip and the strictly-
  increasing-counter pin), totalling 115 in the doc-model crate
  (Phase B was 93). cargo-mutants kills 124/125 = **99.20 %** on the
  moat layer; the single survivor is the equivalent mutant in
  `handle_split_block` carried over from Phase B and already
  documented inline. cargo-llvm-cov reports 97.29 % region / 98.17 %
  line; the 9 uncovered lines are all intentional defensive arms
  (`has_mark` per-Loro-contract, malformed-entry fall-throughs in
  the `read_*_field` helpers + comment/suggestion accessors, and
  the `let-else` no-newline-found fallback in `handle_merge_blocks`).
  Phase D (citations + footnotes — variants 10-11) remains
  deferred; resumption plan lives in the project memory.
- Phase 2 prompt 2.2 — doc-model block model (Phase B — InsertBlock /
  SplitBlock / MergeBlocks): the `apalabrar-doc-model` crate grows a
  block-aware layer on top of the Phase A `EditOp` dispatcher. The
  body remains a single `LoroText` (single-linearised model — locked
  in the lib.rs header), with `\n` codepoints as block separators
  and a parallel `LoroMovableList` named `"blocks"` storing one
  string-encoded `BlockKind` per block. The decision document is in
  the lib.rs header: multi-container was rejected because it would
  require breaking `Position = usize` and Phase A's already-green
  31 edit-op tests; v0 is single-user so the CRDT-merge advantage
  of multi-container is theoretical. `Doc::block_count()` and
  `Doc::block(idx) -> Option<Block>` accessors complete the read
  surface; `BlockKind::Heading.level` clamps into 1..=6 on encode,
  unknown-string decode falls back to `Paragraph` for forward-compat.
  The three EditOp arms now dispatch to private handlers:
  `handle_insert_block` splits the target block at `at` into
  before/new/after with kind preservation on both halves;
  `handle_split_block` inserts a `\n` and duplicates the parent
  kind; `handle_merge_blocks` checks adjacency, finds the separating
  `\n` via a single-pass loop, deletes both the `\n` and the second
  block's kind entry (left-wins). Out-of-bounds positions clip
  defensively per the Phase A contract; non-adjacent or same-block
  merges are no-ops. The `Error::NotYetImplemented` variants for
  these three ops are removed from the dispatcher; the bridge
  contract for those three is now final. Phase A's three NYI tests
  for these variants were rewritten as happy-path tests (justified
  under "tests-are-immutable except when wrong" because the previous
  assertions were placeholder pins, not behavioural invariants);
  `prop_deferred_variants_preserve_text` is updated to skip the now-
  implemented variants and cover only the 5 still-deferred
  comment/suggestion/citation/footnote ops. 67 GREEN in
  `tests/edit_ops.rs` (62 from Phase A + 5 new mutation-killing
  multi-block tests; total of 30 brand-new Phase B tests across
  block accessors, InsertBlock variants, SplitBlock, MergeBlocks,
  snapshot round-trip preserves block kinds, and three properties:
  `prop_block_count_matches_newlines_plus_one`,
  `prop_split_then_merge_restores_text`,
  `prop_insert_block_increments_count`). cargo-mutants kills 75/76
  = **98.68 %** on the doc-model crate (well above the 95 % moat
  floor); the single survivor is a true equivalent mutant in
  `handle_split_block` documented inline. cargo-llvm-cov reports
  99.62 % region / 100 % function / 99.28 % line; the two uncovered
  lines are the pre-existing `else { continue; }` arm in `has_mark`
  (Gate 3) and the `let-else` fallback for the unreachable
  no-newline-found case in `handle_merge_blocks`, both intentional
  defensive arms. Phase C (comments + suggestions — variants 7-9)
  and Phase D (citations + footnotes — variants 10-11) remain
  deferred; resumption plan lives in the project memory.
- Phase 2 prompt 2.2 — doc-model edit-op surface (Phase A — text ops):
  `apalabrar-doc-model` now exposes `EditOp` (the cross-boundary edit
  verb) with all 11 variants from `blueprint-part3-synthesis.md`
  Section G declared up-front (`InsertText`, `DeleteRange`,
  `FormatRange`, `InsertBlock`, `SplitBlock`, `MergeBlocks`,
  `InsertComment`, `Suggest`, `AcceptSuggestion`, `InsertCitation`,
  `InsertFootnote`), plus the supporting value types `Block`,
  `BlockKind`, `BlockTree`, the `Position` alias, and a
  `#[non_exhaustive] Error` enum (currently `NotYetImplemented(name:
  &'static str)`). `Doc::apply_edit_op(&mut self, op: EditOp) ->
  Result<(), Error>` is the dispatcher. Phase A implements the three
  text-level variants by routing to the existing `insert/delete/
  format` Loro path; the remaining eight return
  `Err(Error::NotYetImplemented("VariantName"))` so the bridge
  contract is final from day one and the JS bridge can feature-flag
  variants by inspecting which return that error. 36 new tests pass
  GREEN in `tests/edit_ops.rs` (1 lib + 20 api carry-over + 31
  edit-op tests + 5 CRDT properties; 31 of the 36 are new): 8 happy
  paths for the implemented variants (including LATAM multibyte +
  multi-mark), 13 edge cases (insert-at-end, past-end clip, empty
  text, empty marks, inverted range, empty range, full delete),
  8 NotYetImplemented assertions with the variant-name string
  pinned, 1 deferred-variants-leave-doc-untouched probe, and 3
  `proptest` properties (insert↔delete inverse round-trip over 256
  cases, empty-text idempotence, deferred-variant preservation
  across all 8 deferred variants). cargo-mutants kills 29/29 = **100%**
  on the doc-model crate (target ≥ 95% per moat-layer rule).
  cargo-llvm-cov reports 99.52 % region / 100 % function / 99.15 %
  line on `doc-model/src/lib.rs`; the single uncovered line is a
  pre-existing defensive `else { continue; }` arm in `has_mark`
  that is dead per Loro's `to_delta` contract (documented as
  intentional in the Gate 3 memory). The Phase A-introduced code
  (`apply_edit_op` + `EditOp` dispatcher + value types) is at 100 %
  line coverage. Phase B (block model — variants 4-6), Phase C
  (comments + suggestions — variants 7-9), and Phase D (citations
  + footnotes — variants 10-11) remain deferred; the resumption
  plan lives in the project memory.
- Phase 2 prompt 2.1 — Storage layer (OPFS + IndexedDB + WAL): the
  `@apalabrar/editor-bridge` package now ships a `StorageBackend`
  contract with two implementations. `MemoryStorage` is the
  in-memory reference (defensive-copies blobs on save and load) and
  doubles as the model for the property suite. `OpfsStorage` is the
  durable production backend — blobs in OPFS under
  `blobs/<docId>.bin`, metadata in IndexedDB
  (`apalabrar-storage/metadata`), a WAL on OPFS under
  `wal/<txnId>.json`. The save flow records intent → writes the
  blob (commit on `FileSystemWritableFileStream.close()`) → updates
  the IDB row → drops the WAL record. On construction OpfsStorage
  scans the WAL and reconciles every residual record through the
  pure `decideRecovery` state machine (rollback / commit /
  apply-delete / cleanup). Surface: `parseDocId`, `isDocId`,
  `StorageError` (kinds `invalid-id` | `quota-exceeded` |
  `corruption` | `backend-unavailable`), `MemoryStorage`,
  `OpfsStorage.create()`. 59 unit tests pass GREEN
  (storage.test.ts 14 + wal.test.ts 12 + storage-memory.test.ts 26
  + storage-properties.test.ts 6 + index.test.ts 1) at 100 % line /
  100 % function / 98.82 % branch coverage on the Node-tested
  surface (storage-opfs.ts is browser-only and excluded). 7
  Playwright tests in `tests-e2e/tests/storage.spec.ts` exercise
  OpfsStorage in real Chromium: save→close→reopen→load,
  list/delete, subscribe, invalid-id rejection, crash recovery for
  partial blob (rollback) and complete blob (commit), and the
  Demo page Save/Load/Delete flow. The `/demo` route now exposes
  Save/Load/Delete buttons wired to OpfsStorage with a saved-docs
  panel; the `/storage-harness` route is a Playwright-only
  testing harness that exposes the storage instance on
  `window.__opfs`.
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
