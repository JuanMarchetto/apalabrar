// Contract tests for `ApalabrarCore` (Phase 2.3 JS↔Rust bridge facade).
//
// These tests use a hand-rolled wasm mock that records every call so
// we can assert the JSON payload shape, dispatch order, and return-
// value handling. End-to-end tests against the real wasm pkg run in
// a separate `core.e2e.test.ts` once `pnpm wasm:build` is wired in CI.

import fc from 'fast-check';
import { describe, expect, it } from 'vitest';

import { ApalabrarCore, type ApalabrarCoreWasm, type Block, type EditOp } from './core';

interface CallLog {
  fn: string;
  args: unknown[];
}

class WasmMock implements ApalabrarCoreWasm {
  log: CallLog[] = [];
  nextDocId = 1n;
  blockJsonByIdx = new Map<number, string>();
  textByDoc = new Map<bigint, string>();
  countByDoc = new Map<bigint, number>();
  snapshotBytes = new Uint8Array([42, 17, 99]);

  private record(fn: string, args: unknown[]): void {
    this.log.push({ fn, args });
  }

  createDoc(): bigint {
    this.record('createDoc', []);
    return this.nextDocId++;
  }

  applyEditOp(docId: bigint, opJson: string): void {
    this.record('applyEditOp', [docId, opJson]);
  }

  bridgeSnapshot(docId: bigint): Uint8Array {
    this.record('bridgeSnapshot', [docId]);
    return this.snapshotBytes;
  }

  restoreFromSnapshot(snapshot: Uint8Array): bigint {
    this.record('restoreFromSnapshot', [snapshot]);
    return this.nextDocId++;
  }

  blockCount(docId: bigint): number {
    this.record('blockCount', [docId]);
    return this.countByDoc.get(docId) ?? 1;
  }

  blockAt(docId: bigint, idx: number): string | undefined {
    this.record('blockAt', [docId, idx]);
    return this.blockJsonByIdx.get(idx);
  }

  bridgeDocText(docId: bigint): string {
    this.record('bridgeDocText', [docId]);
    return this.textByDoc.get(docId) ?? '';
  }

  bridgeCloseDoc(docId: bigint): void {
    this.record('bridgeCloseDoc', [docId]);
  }
}

describe('ApalabrarCore', () => {
  describe('lifecycle', () => {
    it('createDoc returns a fresh CoreDocId', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      expect(id).toBe(1n);
      expect(wasm.log).toEqual([{ fn: 'createDoc', args: [] }]);
    });

    it('createDoc returns distinct ids across calls', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const a = core.createDoc();
      const b = core.createDoc();
      expect(a).not.toBe(b);
    });

    it('closeDoc forwards to the wasm export', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      core.closeDoc(id);
      expect(wasm.log[1]).toEqual({ fn: 'bridgeCloseDoc', args: [id] });
    });

    it('snapshot returns the wasm bytes', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      const bytes = core.snapshot(id);
      expect(bytes).toBe(wasm.snapshotBytes);
    });

    it('restoreFromSnapshot returns a fresh id', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.restoreFromSnapshot(new Uint8Array([1, 2, 3]));
      expect(id).toBe(1n);
      expect(wasm.log[0]?.fn).toBe('restoreFromSnapshot');
    });
  });

  describe('applyEditOp dispatch', () => {
    function dispatched(op: EditOp): string {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      core.applyEditOp(id, op);
      const applied = wasm.log.find((c) => c.fn === 'applyEditOp');
      expect(applied).toBeDefined();
      if (!applied) throw new Error('unreachable: defined just asserted');
      return applied.args[1] as string;
    }

    it('serializes InsertText with marks', () => {
      const json = dispatched({
        kind: 'InsertText',
        at: 0,
        text: 'hello',
        marks: ['Bold', 'Italic'],
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'InsertText',
        at: 0,
        text: 'hello',
        marks: ['Bold', 'Italic'],
      });
    });

    it('serializes DeleteRange', () => {
      const json = dispatched({ kind: 'DeleteRange', from: 0, to: 5 });
      expect(JSON.parse(json)).toEqual({
        kind: 'DeleteRange',
        from: 0,
        to: 5,
      });
    });

    it('serializes FormatRange with mark', () => {
      const json = dispatched({
        kind: 'FormatRange',
        from: 0,
        to: 3,
        mark: 'Bold',
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'FormatRange',
        from: 0,
        to: 3,
        mark: 'Bold',
      });
    });

    it('serializes InsertBlock with nested BlockKind', () => {
      const json = dispatched({
        kind: 'InsertBlock',
        at: 0,
        block: {
          kind: { type: 'Heading', level: 2 },
          text: 'title',
        },
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'InsertBlock',
        at: 0,
        block: {
          kind: { type: 'Heading', level: 2 },
          text: 'title',
        },
      });
    });

    it('serializes SplitBlock', () => {
      const json = dispatched({ kind: 'SplitBlock', at: 5 });
      expect(JSON.parse(json)).toEqual({ kind: 'SplitBlock', at: 5 });
    });

    it('serializes MergeBlocks', () => {
      const json = dispatched({ kind: 'MergeBlocks', first: 0, second: 6 });
      expect(JSON.parse(json)).toEqual({
        kind: 'MergeBlocks',
        first: 0,
        second: 6,
      });
    });

    it('serializes InsertComment with explicit thread_id', () => {
      const json = dispatched({
        kind: 'InsertComment',
        from: 0,
        to: 5,
        body: 'review',
        thread_id: 't-1',
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'InsertComment',
        from: 0,
        to: 5,
        body: 'review',
        thread_id: 't-1',
      });
    });

    it('serializes Suggest', () => {
      const json = dispatched({
        kind: 'Suggest',
        from: 0,
        to: 5,
        replacement: 'X',
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'Suggest',
        from: 0,
        to: 5,
        replacement: 'X',
      });
    });

    it('serializes AcceptSuggestion', () => {
      const json = dispatched({
        kind: 'AcceptSuggestion',
        suggestion_id: 's-abc',
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'AcceptSuggestion',
        suggestion_id: 's-abc',
      });
    });

    it('serializes InsertCitation', () => {
      const json = dispatched({
        kind: 'InsertCitation',
        at: 5,
        key: 'Smith2020',
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'InsertCitation',
        at: 5,
        key: 'Smith2020',
      });
    });

    it('serializes InsertFootnote with BlockTree body', () => {
      const json = dispatched({
        kind: 'InsertFootnote',
        at: 5,
        body: {
          blocks: [
            { kind: { type: 'Paragraph' }, text: 'note' },
            { kind: { type: 'Heading', level: 1 }, text: 'sub' },
          ],
        },
      });
      expect(JSON.parse(json)).toEqual({
        kind: 'InsertFootnote',
        at: 5,
        body: {
          blocks: [
            { kind: { type: 'Paragraph' }, text: 'note' },
            { kind: { type: 'Heading', level: 1 }, text: 'sub' },
          ],
        },
      });
    });
  });

  describe('queries', () => {
    it('docText returns the wasm string', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      wasm.textByDoc.set(id, 'hello');
      expect(core.docText(id)).toBe('hello');
    });

    it('blockCount forwards to wasm', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      wasm.countByDoc.set(id, 3);
      expect(core.blockCount(id)).toBe(3);
    });

    it('blockAt parses the wasm JSON into a typed Block', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      wasm.blockJsonByIdx.set(
        0,
        JSON.stringify({
          kind: { type: 'Heading', level: 2 },
          text: 'title',
        }),
      );
      const block = core.blockAt(id, 0);
      const expected: Block = {
        kind: { type: 'Heading', level: 2 },
        text: 'title',
      };
      expect(block).toEqual(expected);
    });

    it('blockAt returns null when wasm returns undefined', () => {
      const wasm = new WasmMock();
      const core = new ApalabrarCore(wasm);
      const id = core.createDoc();
      // No entry in blockJsonByIdx → mock returns undefined.
      expect(core.blockAt(id, 999)).toBeNull();
    });
  });

  describe('properties', () => {
    it('any EditOp serialises to JSON that round-trips through parse', () => {
      fc.assert(
        fc.property(arbitraryEditOp(), (op) => {
          const wasm = new WasmMock();
          const core = new ApalabrarCore(wasm);
          const id = core.createDoc();
          core.applyEditOp(id, op);
          const applied = wasm.log.find((c) => c.fn === 'applyEditOp');
          const json = applied?.args[1] as string;
          expect(JSON.parse(json)).toEqual(op);
        }),
      );
    });
  });
});

function arbitraryMark(): fc.Arbitrary<'Bold' | 'Italic'> {
  return fc.constantFrom('Bold' as const, 'Italic' as const);
}

function arbitraryBlockKind(): fc.Arbitrary<Block['kind']> {
  return fc.oneof(
    fc.constant<Block['kind']>({ type: 'Paragraph' }),
    fc.integer({ min: 1, max: 6 }).map<Block['kind']>((level) => ({
      type: 'Heading',
      level,
    })),
    fc.integer({ min: 0, max: 8 }).map<Block['kind']>((indent) => ({
      type: 'ListItem',
      indent,
    })),
  );
}

function arbitraryBlock(): fc.Arbitrary<Block> {
  return fc.record({
    kind: arbitraryBlockKind(),
    text: fc.string({ maxLength: 12 }),
  });
}

function arbitraryEditOp(): fc.Arbitrary<EditOp> {
  const pos = fc.integer({ min: 0, max: 50 });
  return fc.oneof(
    fc
      .record({
        at: pos,
        text: fc.string({ maxLength: 12 }),
        marks: fc.array(arbitraryMark(), { maxLength: 2 }),
      })
      .map<EditOp>((r) => ({ kind: 'InsertText', ...r })),
    fc
      .record({ from: pos, to: pos })
      .map<EditOp>((r) => ({ kind: 'DeleteRange', ...r })),
    fc
      .record({ from: pos, to: pos, mark: arbitraryMark() })
      .map<EditOp>((r) => ({ kind: 'FormatRange', ...r })),
    fc
      .record({ at: pos, block: arbitraryBlock() })
      .map<EditOp>((r) => ({ kind: 'InsertBlock', ...r })),
    pos.map<EditOp>((at) => ({ kind: 'SplitBlock', at })),
    fc
      .record({ first: pos, second: pos })
      .map<EditOp>((r) => ({ kind: 'MergeBlocks', ...r })),
    fc
      .record({
        from: pos,
        to: pos,
        body: fc.string({ maxLength: 12 }),
        thread_id: fc.option(fc.string({ minLength: 1, maxLength: 8 })),
      })
      .map<EditOp>((r) => ({ kind: 'InsertComment', ...r })),
    fc
      .record({
        from: pos,
        to: pos,
        replacement: fc.string({ maxLength: 12 }),
      })
      .map<EditOp>((r) => ({ kind: 'Suggest', ...r })),
    fc
      .string({ minLength: 1, maxLength: 12 })
      .map<EditOp>((id) => ({ kind: 'AcceptSuggestion', suggestion_id: id })),
    fc
      .record({ at: pos, key: fc.string({ minLength: 1, maxLength: 12 }) })
      .map<EditOp>((r) => ({ kind: 'InsertCitation', ...r })),
    fc
      .record({
        at: pos,
        body: fc.record({ blocks: fc.array(arbitraryBlock(), { maxLength: 3 }) }),
      })
      .map<EditOp>((r) => ({ kind: 'InsertFootnote', ...r })),
  );
}
