// Phase 5.6.1 — runtime singleton tests.
//
// `runtime.ts` is the editor-bridge entry point that orchestrates the
// one-time wasm initialisation and exposes the resulting
// `ApalabrarCore` to the rest of the app. The contract these tests
// pin down:
//
//   - `getCore()` MUST throw if called before `initWasm()` resolves.
//   - `initWasm({ wasm })` skips the real wasm-pack init and uses the
//     supplied module directly. This is the seam the test suite hangs
//     on; production callers must not pass `wasm`.
//   - `initWasm()` is idempotent — concurrent calls share a promise,
//     and a second call after the first resolves never replaces the
//     cached instance.
//   - `__resetCoreForTests()` clears the singleton so tests stay
//     independent.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { ApalabrarCore, type ApalabrarCoreWasm } from './core';
import { __resetCoreForTests, getCore, initWasm } from './runtime';

class StubWasm implements ApalabrarCoreWasm {
  createDoc(): bigint {
    return 1n;
  }
  applyEditOp(): void {}
  bridgeSnapshot(): Uint8Array {
    return new Uint8Array();
  }
  restoreFromSnapshot(): bigint {
    return 1n;
  }
  blockCount(): number {
    return 0;
  }
  blockAt(): string | undefined {
    return undefined;
  }
  bridgeDocText(): string {
    return '';
  }
  bridgeCloseDoc(): void {}
  findInDoc(): string {
    return '[]';
  }
  commentsInDoc(): string {
    return '[]';
  }
  suggestionsInDoc(): string {
    return '[]';
  }
  footnotesInDoc(): string {
    return '[]';
  }
  openDoc(): bigint {
    return 1n;
  }
  toFormat(): Uint8Array {
    return new Uint8Array();
  }
}

describe('editor-bridge runtime', () => {
  beforeEach(() => __resetCoreForTests());
  afterEach(() => __resetCoreForTests());

  it('getCore throws when called before initWasm', () => {
    expect(() => getCore()).toThrowError(
      /getCore\(\) called before initWasm/,
    );
  });

  it('initWasm with injected wasm resolves to an ApalabrarCore', async () => {
    const wasm = new StubWasm();
    const core = await initWasm({ wasm });
    expect(core).toBeInstanceOf(ApalabrarCore);
  });

  it('getCore returns the same instance after initWasm resolves', async () => {
    const wasm = new StubWasm();
    const core = await initWasm({ wasm });
    expect(getCore()).toBe(core);
  });

  it('concurrent initWasm calls share a single promise', async () => {
    const wasm = new StubWasm();
    const a = initWasm({ wasm });
    const b = initWasm({ wasm });
    expect(await a).toBe(await b);
  });

  it('subsequent initWasm calls keep the first instance', async () => {
    const first = await initWasm({ wasm: new StubWasm() });
    const second = await initWasm({ wasm: new StubWasm() });
    expect(second).toBe(first);
  });

  it('the wired ApalabrarCore dispatches through the injected wasm', async () => {
    const wasm = new StubWasm();
    const core = await initWasm({ wasm });
    const id = core.createDoc();
    // The doc id from StubWasm is hard-coded to 1n; this proves the
    // facade actually delegates to the injected module.
    expect(id).toBe(1n);
  });
});
