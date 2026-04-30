/* tslint:disable */
/* eslint-disable */

export function applyDelete(doc_id: bigint, start: number, end: number): void;

export function applyEditOp(doc_id: bigint, op_json: string): void;

export function applyInsert(doc_id: bigint, offset: number, text: string): void;

export function blockAt(doc_id: bigint, idx: number): string | undefined;

export function blockCount(doc_id: bigint): number;

export function bridgeCloseDoc(doc_id: bigint): void;

export function bridgeDocText(doc_id: bigint): string;

export function bridgeSnapshot(doc_id: bigint): Uint8Array;

export function closeDoc(doc_id: bigint): void;

export function commentsInDoc(doc_id: bigint): string;

export function createDoc(): bigint;

export function docText(doc_id: bigint): string;

export function findInDoc(doc_id: bigint, needle: string, opts_json: string): string;

export function footnotesInDoc(doc_id: bigint): string;

/**
 * Phase 5.6.2 — multi-format open. `format` is one of
 * `"docx"`, `"md"` (alias `"markdown"`), `"html"`, `"rtf"`,
 * `"odt"`. Unknown identifiers yield `Error::UnknownFormat`.
 */
export function openDoc(bytes: Uint8Array, format: string): bigint;

export function openDocx(bytes: Uint8Array): bigint;

export function restoreFromSnapshot(snap: Uint8Array): bigint;

export function suggestionsInDoc(doc_id: bigint): string;

export function toDocx(doc_id: bigint): Uint8Array;

/**
 * Phase 5.6.2 — symmetric multi-format save. Returns the
 * serialised bytes for the requested format, or surfaces a
 * `FormatNotSupported` / `UnknownFormat` error.
 */
export function toFormat(doc_id: bigint, format: string): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly applyDelete: (a: bigint, b: number, c: number) => [number, number];
    readonly applyEditOp: (a: bigint, b: number, c: number) => [number, number];
    readonly applyInsert: (a: bigint, b: number, c: number, d: number) => [number, number];
    readonly blockAt: (a: bigint, b: number) => [number, number, number, number];
    readonly blockCount: (a: bigint) => [number, number, number];
    readonly bridgeCloseDoc: (a: bigint) => [number, number];
    readonly bridgeDocText: (a: bigint) => [number, number, number, number];
    readonly bridgeSnapshot: (a: bigint) => [number, number, number, number];
    readonly commentsInDoc: (a: bigint) => [number, number, number, number];
    readonly findInDoc: (a: bigint, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly footnotesInDoc: (a: bigint) => [number, number, number, number];
    readonly openDoc: (a: number, b: number, c: number, d: number) => [bigint, number, number];
    readonly openDocx: (a: number, b: number) => [bigint, number, number];
    readonly restoreFromSnapshot: (a: number, b: number) => [bigint, number, number];
    readonly suggestionsInDoc: (a: bigint) => [number, number, number, number];
    readonly toDocx: (a: bigint) => [number, number, number, number];
    readonly toFormat: (a: bigint, b: number, c: number) => [number, number, number, number];
    readonly createDoc: () => bigint;
    readonly docText: (a: bigint) => [number, number, number, number];
    readonly closeDoc: (a: bigint) => [number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
