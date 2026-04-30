// Phase 5.6.1 — wasm runtime bootstrap.
//
// `initWasm` wraps the wasm-pack glue's `init({ module_or_path })`
// and constructs a singleton `ApalabrarCore` facade. App-side code
// then uses `getCore()` to dispatch through the typed bridge:
//
//     await initWasm();           // once, at startup
//     const core = getCore();
//     const id = core.createDoc();
//     core.applyEditOp(id, op);
//
// The `wasm` option is the test-injection seam — supplying a stubbed
// `ApalabrarCoreWasm` skips the real wasm load. Production callers
// must not pass it.

import { ApalabrarCore, type ApalabrarCoreWasm } from './core';

export interface InitWasmOptions {
  /** Override the wasm binary URL. Defaults to the bundled
   *  `apalabrar-editor-core` binary served by Vite. */
  readonly wasmUrl?: string;
  /** Test-only injection: skip the real wasm-pack init and wrap the
   *  supplied module directly. Production code must not pass this. */
  readonly wasm?: ApalabrarCoreWasm;
}

let initPromise: Promise<ApalabrarCore> | null = null;
let coreInstance: ApalabrarCore | null = null;

/**
 * Initialise the editor wasm core. Idempotent: concurrent and
 * subsequent calls share the first call's promise and resolved
 * instance. Returns the cached `ApalabrarCore` once ready.
 */
export async function initWasm(
  options: InitWasmOptions = {},
): Promise<ApalabrarCore> {
  if (initPromise) return initPromise;
  initPromise = (async () => {
    const wasm = options.wasm ?? (await loadProductionWasm(options.wasmUrl));
    const core = new ApalabrarCore(wasm);
    coreInstance = core;
    return core;
  })();
  return initPromise;
}

/**
 * Synchronous accessor for the post-init singleton. Throws if called
 * before `initWasm()` resolves; this is a programming error — the app
 * shell must `await initWasm()` once before any consumer calls
 * `getCore()`.
 */
export function getCore(): ApalabrarCore {
  if (!coreInstance) {
    throw new Error(
      'apalabrar-editor-bridge: getCore() called before initWasm() resolved. ' +
        'Call `await initWasm()` once at app startup.',
    );
  }
  return coreInstance;
}

/** Test-only: clear the singleton so each test starts fresh. */
export function __resetCoreForTests(): void {
  initPromise = null;
  coreInstance = null;
}

async function loadProductionWasm(
  wasmUrl: string | undefined,
): Promise<ApalabrarCoreWasm> {
  // Dynamic import keeps the wasm out of the test-side dependency
  // graph — when tests pass `{ wasm }` to `initWasm`, this branch
  // never runs and the heavy wasm-pack module isn't pulled in.
  const mod = await import('apalabrar-editor-core');
  const url = wasmUrl ??
    (
      await import('apalabrar-editor-core/apalabrar_editor_core_bg.wasm?url')
    ).default;
  await mod.default({ module_or_path: url });
  return mod as unknown as ApalabrarCoreWasm;
}
