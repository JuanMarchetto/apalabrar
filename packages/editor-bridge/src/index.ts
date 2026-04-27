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

export const VERSION = '0.0.0' as const;
