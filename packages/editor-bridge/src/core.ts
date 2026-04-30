// Apalabrar JS↔Rust bridge — Phase 2.3.
//
// `ApalabrarCore` wraps the wasm exports from `apalabrar-editor-core`
// with a typed object surface. Each method takes a typed argument
// (eg. an `EditOp` discriminated union) and dispatches to the
// underlying wasm function, JSON-encoding complex payloads at the
// boundary.
//
// The wasm module is INJECTED via the constructor so tests can pass
// a mock and so callers can defer wasm initialization. In production
// the caller does:
//
//   import * as wasm from 'apalabrar-editor-core';
//   await init({ module_or_path: wasmUrl });
//   const core = new ApalabrarCore(wasm);
//
// The interface is a *strict subset* of the blueprint Section G
// `ApalabrarCore` declaration. Phase 2.3 ships:
//   - lifecycle: createDoc, closeDoc, snapshot, restoreFromSnapshot
//   - ops: applyEditOp (covers all 11 doc-model EditOp variants)
//   - queries: docText, blockCount, blockAt
// The remaining Section G methods (collab, AI, plugins, search,
// rich-text getRange, render plan, format I/O beyond OOXML) ship in
// later prompts as their underlying crates come online.

// ─────────────────────────────────────────────────────────────────
// Wire types — match the doc-model serde shape exactly.
// ─────────────────────────────────────────────────────────────────

export type Position = number;

export type Mark = 'Bold' | 'Italic';

export type BlockKind =
  | { type: 'Paragraph'; }
  | { type: 'Heading'; level: number; }
  | { type: 'ListItem'; indent: number; };

export interface Block {
  kind: BlockKind;
  text: string;
}

export interface BlockTree {
  blocks: Block[];
}

/** Phase 4.6 — comment thread lifecycle. Matches Rust serde
 * `#[serde(rename_all = "lowercase")] CommentStatus`. */
export type CommentStatus = 'open' | 'resolved';

export type EditOp =
  | { kind: 'InsertText'; at: Position; text: string; marks: Mark[]; }
  | { kind: 'DeleteRange'; from: Position; to: Position; }
  | { kind: 'FormatRange'; from: Position; to: Position; mark: Mark; }
  | { kind: 'InsertBlock'; at: Position; block: Block; }
  | { kind: 'SplitBlock'; at: Position; }
  | { kind: 'MergeBlocks'; first: Position; second: Position; }
  | {
    kind: 'InsertComment';
    from: Position;
    to: Position;
    body: string;
    thread_id: string | null;
    author: string;
    /** Epoch milliseconds. */
    created_at: number;
  }
  | {
    kind: 'ReplyToComment';
    thread_id: string;
    body: string;
    author: string;
    /** Epoch milliseconds. */
    created_at: number;
  }
  | { kind: 'SetCommentStatus'; thread_id: string; status: CommentStatus; }
  | {
    kind: 'Suggest';
    from: Position;
    to: Position;
    replacement: string;
    author: string;
    /** Epoch milliseconds. */
    created_at: number;
  }
  | { kind: 'AcceptSuggestion'; suggestion_id: string; }
  | { kind: 'RejectSuggestion'; suggestion_id: string; }
  | { kind: 'InsertCitation'; at: Position; key: string; }
  | { kind: 'InsertFootnote'; at: Position; body: BlockTree; };

/** Phase 4.6 — one reply inside a comment thread. */
export interface CommentReply {
  id: string;
  body: string;
  author: string;
  /** Epoch milliseconds. */
  created_at: number;
}

/**
 * Phase 4.7 — suggestion lifecycle. Lowercase string union matches
 * Rust's `#[serde(rename_all = "lowercase")] SuggestionState`.
 */
export type SuggestionState = 'pending' | 'accepted' | 'rejected';

/** Phase 4.7 — suggestion snapshot returned by `core.suggestions()`. */
export interface Suggestion {
  id: string;
  /** Original suggestion range (codepoints) at creation time. */
  from: Position;
  /** Original suggestion range end (exclusive). */
  to: Position;
  /** The replacement text that AcceptSuggestion would apply. */
  replacement: string;
  state: SuggestionState;
  author: string;
  /** Epoch milliseconds. */
  created_at: number;
}

/** Phase 4.6 — comment thread snapshot returned by `core.comment(id)`. */
export interface Comment {
  thread_id: string;
  from: Position;
  to: Position;
  body: string;
  author: string;
  /** Epoch milliseconds. */
  created_at: number;
  status: CommentStatus;
  replies: CommentReply[];
}

/**
 * Phase 5.1 — footnote snapshot returned by `core.footnotes()`.
 * `at` is the codepoint position of the `\u{E001}` marker in body
 * at creation time; the marker (and its mark-anchored id) drift
 * with subsequent edits — to read the CURRENT marker position use
 * the layout `FootnoteRef`. `body` is the footnote contents.
 */
export interface Footnote {
  id: string;
  at: Position;
  body: BlockTree;
}

/// Branded handle returned by `createDoc` / `restoreFromSnapshot`.
/// Wraps a `bigint` (the wasm-bindgen u64 representation) so
/// callers can't accidentally swap a doc id for any other bigint.
export type CoreDocId = bigint & { readonly __brand: 'CoreDocId'; };

/**
 * Phase 5.6.2 — supported document formats for `openDoc` / `toFormat`.
 *
 * - `'docx'` and `'markdown'` round-trip through the editor in v0.
 * - `'html'` is read/written as UTF-8 source in v0.
 * - `'rtf'` and `'odt'` are recognised but their crates are stubs;
 *   the wasm side returns `FormatNotSupported` so the UI can show a
 *   "deferred to v1" message rather than a generic parse error.
 */
export type DocFormat = 'docx' | 'markdown' | 'html' | 'rtf' | 'odt';

// ─────────────────────────────────────────────────────────────────
// Phase 5.7 — Layout types (mirror of `apalabrar_layout::RenderPlan`).
// All field names are camelCase to match the Rust serde wire format.
// ─────────────────────────────────────────────────────────────────

/** Page geometry in logical pixels. The 96-DPI Letter default
 *  matches the Rust constant `LETTER_AT_96DPI`. */
export interface Viewport {
  pageWidthPx: number;
  pageHeightPx: number;
  marginPx: number;
}

/** 96-DPI Letter (8.5" × 11") with 1" margins. The default the
 *  app shell hands to `core.layout()` until the user picks a
 *  different page size. */
export const LETTER_AT_96DPI: Viewport = {
  pageWidthPx: 816,
  pageHeightPx: 1056,
  marginPx: 96,
};

/** Axis-aligned rectangle in viewport pixels. Origin is top-left
 *  of the page's printable area. */
export interface LayoutRect {
  xPx: number;
  yPx: number;
  widthPx: number;
  heightPx: number;
}

/** Layout-side block kind. Mirrors `BlockKind` from the layout
 *  crate; the doc-model has its own (compatible) enum. */
export type LayoutBlockKind =
  | { type: 'Paragraph'; }
  | { type: 'Heading'; level: number; }
  | { type: 'ListItem'; indent: number; };

/** One rendered line. Coordinates are relative to the parent
 *  block's origin. */
export interface LayoutLine {
  widthPx: number;
  heightPx: number;
  baselineYPx: number;
}

/** A `Range<usize>` from Rust serialises as `{ start, end }`. */
export interface IndexRange {
  start: number;
  end: number;
}

/** One placement of a block on a page. A block split across pages
 *  produces multiple `BlockBox`es with the same `blockIndex`. */
export interface BlockBox {
  blockIndex: number;
  kind: LayoutBlockKind;
  originXPx: number;
  originYPx: number;
  widthPx: number;
  heightPx: number;
  lines: LayoutLine[];
  lineRange: IndexRange;
}

/** A footnote body shelf entry on a page. */
export interface FootnoteBox {
  footnoteId: string;
  displayNumber: number;
  originXPx: number;
  originYPx: number;
  widthPx: number;
  heightPx: number;
  lines: LayoutLine[];
  isContinuation: boolean;
}

/** Body-side anchor for a footnote marker. */
export interface FootnoteRef {
  footnoteId: string;
  displayNumber: number;
  pageIndex: number;
  blockIndex: number;
  lineIndex: number;
  xPx: number;
  baselineYPx: number;
}

/** One page of laid-out blocks + footnote shelf. `pageNumber` is
 *  1-indexed (academic citation convention). */
export interface Page {
  blocks: BlockBox[];
  pageNumber: number;
  footnotes: FootnoteBox[];
}

/** One shaped glyph in line-local coordinates. */
export interface PositionedGlyph {
  glyphId: number;
  clusterStart: number;
  clusterEnd: number;
  xPx: number;
  yPx: number;
  widthPx: number;
}

/** One run of shaped glyphs sharing a single block + line + font
 *  size. Phase 4.2 emits exactly one per shaped line. */
export interface GlyphRun {
  blockIndex: number;
  lineIndex: number;
  fontSizePx: number;
  baselineYPx: number;
  glyphs: PositionedGlyph[];
}

/** Output of `core.layout()`: the full RenderPlan the painter
 *  consumes per doc revision. */
export interface RenderPlan {
  pages: Page[];
  dirtyRects: LayoutRect[];
  glyphRuns: GlyphRun[];
  footnoteRefs: FootnoteRef[];
}

// ─────────────────────────────────────────────────────────────────
// Wasm module interface — the exports `apalabrar-editor-core` will
// expose once the pkg is rebuilt with the Phase 2.3 bridge module.
// Defined here as an interface so tests can supply a mock without
// touching the actual wasm binary.
// ─────────────────────────────────────────────────────────────────

export interface ApalabrarCoreWasm {
  createDoc(): bigint;
  applyEditOp(docId: bigint, opJson: string): void;
  bridgeSnapshot(docId: bigint): Uint8Array;
  restoreFromSnapshot(snapshot: Uint8Array): bigint;
  blockCount(docId: bigint): number;
  blockAt(docId: bigint, idx: number): string | undefined;
  bridgeDocText(docId: bigint): string;
  bridgeCloseDoc(docId: bigint): void;
  /** Phase 4.5 — returns a JSON-encoded `MatchJson[]`. */
  findInDoc(docId: bigint, needle: string, optsJson: string): string;
  /** Phase 4.6 — returns a JSON-encoded `Comment[]`. */
  commentsInDoc(docId: bigint): string;
  /** Phase 4.7 — returns a JSON-encoded `Suggestion[]`. */
  suggestionsInDoc(docId: bigint): string;
  /** Phase 5.1 — returns a JSON-encoded `Footnote[]` in body-position order. */
  footnotesInDoc(docId: bigint): string;
  /** Phase 5.6.2 — multi-format open. `format` is the canonical
   *  short form (`'docx' | 'md' | 'html' | 'rtf' | 'odt'`); the
   *  bridge accepts `'markdown'` as a synonym for `'md'`. */
  openDoc(bytes: Uint8Array, format: string): bigint;
  /** Phase 5.6.2 — symmetric multi-format save. */
  toFormat(docId: bigint, format: string): Uint8Array;
  /** Phase 5.7 — lay out the doc against the given viewport.
   *  Returns the `RenderPlan` as a JSON string the facade parses. */
  layoutDoc(docId: bigint, viewportJson: string): string;
}

// ─────────────────────────────────────────────────────────────────
// Phase 4.5 — Find types
// ─────────────────────────────────────────────────────────────────

/** Search options. Matches the Rust `FindOptionsJson` serde shape. */
export interface FindOptions {
  caseSensitive: boolean;
  wholeWord: boolean;
}

/**
 * One match in the doc's plain-text projection. Half-open
 * `[start, end)` in CODEPOINT units (NOT bytes), matching
 * `Position`.
 */
export interface Match {
  start: Position;
  end: Position;
}

// ─────────────────────────────────────────────────────────────────
// ApalabrarCore — typed JS↔Rust bridge facade.
// ─────────────────────────────────────────────────────────────────

export class ApalabrarCore {
  constructor(private readonly wasm: ApalabrarCoreWasm) {}

  /// Allocate a fresh empty document.
  createDoc(): CoreDocId {
    return this.wasm.createDoc() as CoreDocId;
  }

  /// Drop a document from the registry.
  closeDoc(id: CoreDocId): void {
    this.wasm.bridgeCloseDoc(id);
  }

  /// Apply an `EditOp` to the document. Throws on bad ops (unknown
  /// doc id, malformed payloads); the wasm side returns Ok-or-throws.
  applyEditOp(id: CoreDocId, op: EditOp): void {
    this.wasm.applyEditOp(id, JSON.stringify(op));
  }

  /// Project the doc to a UTF-8 string. Block boundaries appear as
  /// `\n`; citation markers as `\u{E000}`; footnote markers as
  /// `\u{E001}`. The layout engine renders these accordingly.
  docText(id: CoreDocId): string {
    return this.wasm.bridgeDocText(id);
  }

  /// Number of blocks in the doc.
  blockCount(id: CoreDocId): number {
    return this.wasm.blockCount(id);
  }

  /// Block at index, or `null` if out of range.
  blockAt(id: CoreDocId, idx: number): Block | null {
    const json = this.wasm.blockAt(id, idx);
    if (json === undefined) return null;
    return JSON.parse(json) as Block;
  }

  /// Export the doc's CRDT state as a snapshot blob (suitable for
  /// the storage layer's `saveDoc`).
  snapshot(id: CoreDocId): Uint8Array {
    return this.wasm.bridgeSnapshot(id);
  }

  /// Restore a doc from a snapshot blob and return its handle.
  restoreFromSnapshot(snapshot: Uint8Array): CoreDocId {
    return this.wasm.restoreFromSnapshot(snapshot) as CoreDocId;
  }

  /**
   * Find all non-overlapping matches of `needle` in the doc's
   * plain-text projection. Returns codepoint ranges sorted ascending
   * by `start`. An empty `needle` returns `[]`.
   *
   * Phase 4.5 — `regex` mode is deferred to v1; `caseSensitive` and
   * `wholeWord` are the only supported flags.
   */
  find(id: CoreDocId, needle: string, opts: FindOptions): Match[] {
    const json = this.wasm.findInDoc(id, needle, JSON.stringify(opts));
    return JSON.parse(json) as Match[];
  }

  /**
   * Phase 4.6 — list every comment thread in the doc, sorted by
   * thread_id. Returns the snapshot type [`Comment`] which includes
   * status + replies.
   */
  comments(id: CoreDocId): Comment[] {
    return JSON.parse(this.wasm.commentsInDoc(id)) as Comment[];
  }

  /**
   * Phase 4.7 — list every suggestion in the doc, sorted by id.
   * Returns the snapshot type [`Suggestion`] which carries state +
   * author + created_at.
   */
  suggestions(id: CoreDocId): Suggestion[] {
    return JSON.parse(this.wasm.suggestionsInDoc(id)) as Suggestion[];
  }

  /**
   * Phase 5.1 — list every footnote in the doc, in body-position
   * order. The first footnote in the returned array renders as
   * superscript "1"; the second as "2"; etc.
   */
  footnotes(id: CoreDocId): Footnote[] {
    return JSON.parse(this.wasm.footnotesInDoc(id)) as Footnote[];
  }

  /**
   * Phase 5.6.2 — open a document in any supported format. Throws
   * via the wasm boundary on `EmptyInput`, `InvalidInput`, or
   * `FormatNotSupported`; the caller decides how to surface those.
   */
  openDoc(bytes: Uint8Array, format: DocFormat): CoreDocId {
    return this.wasm.openDoc(bytes, format === 'markdown' ? 'md' : format) as CoreDocId;
  }

  /**
   * Phase 5.6.2 — symmetric multi-format save. Returns the bytes
   * the host should hand to the file picker / blob download.
   * Throws on `FormatNotSupported`.
   */
  toFormat(id: CoreDocId, format: DocFormat): Uint8Array {
    return this.wasm.toFormat(id, format === 'markdown' ? 'md' : format);
  }

  /**
   * Phase 5.7 — lay out the doc against `viewport` and return the
   * `RenderPlan` the painter consumes. The wasm side does the
   * shaping + page packing in Rust; this method is a thin
   * JSON crossing.
   */
  layout(id: CoreDocId, viewport: Viewport): RenderPlan {
    const json = this.wasm.layoutDoc(id, JSON.stringify(viewport));
    return JSON.parse(json) as RenderPlan;
  }
}
