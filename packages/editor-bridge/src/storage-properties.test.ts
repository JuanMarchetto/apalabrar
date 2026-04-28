// Property tests for the storage layer. Use the in-memory reference
// backend as the model: any sequence of save/delete operations
// applied to the backend should leave it consistent with the
// equivalent JS Map of operations.
//
// Per the project's TDD plan, properties cover invariants that
// example tests miss — N saves followed by N loads should return
// each saved blob, regardless of save order; load on a deleted
// id always returns null; etc.

import * as fc from 'fast-check';
import { describe, it } from 'vitest';
import { type DocId, type DocMetadata, parseDocId, type StorageBackend } from './storage';
import { MemoryStorage } from './storage-memory';

const docIdArb = fc
  .stringMatching(/^[a-zA-Z0-9_-]{1,16}$/)
  .filter((s) => s.length > 0)
  .map((s) => parseDocId(s));

const blobArb = fc
  .uint8Array({ minLength: 0, maxLength: 256 });

type Op =
  | { kind: 'save'; id: DocId; blob: Uint8Array; }
  | { kind: 'delete'; id: DocId; };

const opArb: fc.Arbitrary<Op> = fc.oneof(
  fc.record({
    kind: fc.constant('save' as const),
    id: docIdArb,
    blob: blobArb,
  }),
  fc.record({
    kind: fc.constant('delete' as const),
    id: docIdArb,
  }),
);

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

async function applyOps(
  backend: StorageBackend,
  ops: Op[],
): Promise<Map<DocId, Uint8Array>> {
  // Reference model: a JS Map updated alongside the backend.
  const model = new Map<DocId, Uint8Array>();
  for (const op of ops) {
    if (op.kind === 'save') {
      await backend.saveDoc(op.id, op.blob);
      model.set(op.id, op.blob);
    } else {
      await backend.deleteDoc(op.id);
      model.delete(op.id);
    }
  }
  return model;
}

describe('property: storage equivalence with reference model', () => {
  it('after a sequence of save/delete, loadDoc returns the model state', async () => {
    await fc.assert(
      fc.asyncProperty(fc.array(opArb, { maxLength: 30 }), async (ops) => {
        const backend = new MemoryStorage();
        const model = await applyOps(backend, ops);

        // Every id mentioned must round-trip through loadDoc.
        const seen = new Set<DocId>();
        for (const op of ops) seen.add(op.id);

        for (const id of seen) {
          const loaded = await backend.loadDoc(id);
          const expected = model.get(id);
          if (expected === undefined) {
            if (loaded !== null) return false;
          } else {
            if (loaded === null) return false;
            if (!bytesEqual(loaded, expected)) return false;
          }
        }
        return true;
      }),
      { numRuns: 100 },
    );
  });

  it('listDocs returns exactly the ids in the model', async () => {
    await fc.assert(
      fc.asyncProperty(fc.array(opArb, { maxLength: 30 }), async (ops) => {
        const backend = new MemoryStorage();
        const model = await applyOps(backend, ops);
        const list = await backend.listDocs();
        const observedIds = new Set(list.map((m: DocMetadata) => m.id));
        const expectedIds = new Set(model.keys());
        if (observedIds.size !== expectedIds.size) return false;
        for (const id of expectedIds) if (!observedIds.has(id)) return false;
        return true;
      }),
      { numRuns: 100 },
    );
  });

  it('reported sizes in listDocs match the saved blob lengths', async () => {
    await fc.assert(
      fc.asyncProperty(fc.array(opArb, { maxLength: 30 }), async (ops) => {
        const backend = new MemoryStorage();
        const model = await applyOps(backend, ops);
        const list = await backend.listDocs();
        for (const meta of list) {
          const expected = model.get(meta.id);
          if (expected === undefined) return false;
          if (meta.sizeBytes !== expected.length) return false;
        }
        return true;
      }),
      { numRuns: 100 },
    );
  });

  it('saveDoc then loadDoc on the same id always roundtrips bytes (idempotence-of-read)', async () => {
    await fc.assert(
      fc.asyncProperty(docIdArb, blobArb, async (id, blob) => {
        const backend = new MemoryStorage();
        await backend.saveDoc(id, blob);
        const loaded = await backend.loadDoc(id);
        if (loaded === null) return false;
        return bytesEqual(loaded, blob);
      }),
      { numRuns: 100 },
    );
  });

  it('deleteDoc then loadDoc always returns null', async () => {
    await fc.assert(
      fc.asyncProperty(docIdArb, blobArb, async (id, blob) => {
        const backend = new MemoryStorage();
        await backend.saveDoc(id, blob);
        await backend.deleteDoc(id);
        return (await backend.loadDoc(id)) === null;
      }),
      { numRuns: 100 },
    );
  });

  it('subscribeChanges fires exactly once per save and once per delete', async () => {
    await fc.assert(
      fc.asyncProperty(fc.array(opArb, { minLength: 1, maxLength: 20 }), async (ops) => {
        const backend = new MemoryStorage();
        const events: DocId[] = [];
        backend.subscribeChanges((id) => events.push(id));
        for (const op of ops) {
          if (op.kind === 'save') await backend.saveDoc(op.id, op.blob);
          else await backend.deleteDoc(op.id);
        }
        // Every operation should produce exactly one notification.
        return events.length === ops.length;
      }),
      { numRuns: 50 },
    );
  });
});
