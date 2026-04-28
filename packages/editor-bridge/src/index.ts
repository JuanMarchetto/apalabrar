// Apalabrar editor-bridge: re-export of the Rust+WASM editor core.
//
// The Rust crate `apalabrar-editor-core` is built by `wasm-pack build
// --target web --release crates/editor-core` into
// `crates/editor-core/pkg/`, which has its own `package.json` named
// `apalabrar-editor-core` and is wired in here as a `file:` dep so the
// pnpm workspace resolves the import without a publish step. Run
// `pnpm wasm:build` (defined at the workspace root) before `pnpm dev`
// to keep this package fresh.

export {
  applyDelete,
  applyInsert,
  closeDoc,
  default as init,
  docText,
  openDocx,
  toDocx,
} from 'apalabrar-editor-core';

// `wasmUrl` is the build-time URL of the .wasm binary. Vite intercepts
// the `?url` suffix and emits the asset with a hashed filename in the
// production bundle (and serves it directly in dev). Pass this URL to
// `init({ module_or_path: wasmUrl })` to bypass the wasm-pack glue's
// default `new URL(..., import.meta.url)` resolution, which Vite cannot
// rewrite because the glue lives in node_modules and the URL points at
// the bundled JS chunk rather than the .wasm asset (manifests as a
// "magic word" CompileError where `<!DO` is the start of an SPA-fallback
// `index.html` response).
export { default as wasmUrl } from 'apalabrar-editor-core/apalabrar_editor_core_bg.wasm?url';

// Storage layer (Phase 2 prompt 2.1). See storage.ts for the contract,
// storage-memory.ts for the in-memory reference, and storage-opfs.ts
// for the production backend (OPFS + IndexedDB + WAL).
export {
  type ChangeListener,
  type DocId,
  type DocMetadata,
  isDocId,
  parseDocId,
  type StorageBackend,
  StorageError,
  type StorageErrorKind,
  type Unsubscribe,
} from './storage';
export { MemoryStorage } from './storage-memory';
export { OpfsStorage } from './storage-opfs';

export const VERSION = '0.0.0' as const;
