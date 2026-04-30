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
  }
  | { kind: 'Suggest'; from: Position; to: Position; replacement: string; }
  | { kind: 'AcceptSuggestion'; suggestion_id: string; }
  | { kind: 'InsertCitation'; at: Position; key: string; }
  | { kind: 'InsertFootnote'; at: Position; body: BlockTree; };

/// Branded handle returned by `createDoc` / `restoreFromSnapshot`.
/// Wraps a `bigint` (the wasm-bindgen u64 representation) so
/// callers can't accidentally swap a doc id for any other bigint.
export type CoreDocId = bigint & { readonly __brand: 'CoreDocId'; };

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
}
